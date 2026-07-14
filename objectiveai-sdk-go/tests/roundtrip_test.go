package tests

import (
	"encoding/json"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// AST-based type info extraction
// ---------------------------------------------------------------------------

type typeInfo struct {
	name           string
	doc            string
	isAlias        bool
	aliasTarget    string
	underlyingType string // for non-struct type definitions (e.g., type Foo *string)
	fields         []fieldInfo
	embeds         []string
	methods        map[string]string
}

type fieldInfo struct {
	name     string
	typeName string
	doc      string
	tags     string
}

func parseSourceDir(t *testing.T) map[string]*typeInfo {
	t.Helper()
	dir := SourceDir()

	fset := token.NewFileSet()
	types := map[string]*typeInfo{}

	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("reading source dir: %v", err)
	}

	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".go") {
			continue
		}
		if strings.HasSuffix(entry.Name(), "_test.go") {
			continue
		}

		f, err := parser.ParseFile(fset, filepath.Join(dir, entry.Name()), nil, parser.ParseComments)
		if err != nil {
			t.Fatalf("parsing %s: %v", entry.Name(), err)
		}

		for _, decl := range f.Decls {
			switch d := decl.(type) {
			case *ast.GenDecl:
				for _, spec := range d.Specs {
					ts, ok := spec.(*ast.TypeSpec)
					if !ok {
						continue
					}
					ti := &typeInfo{
						name:    ts.Name.Name,
						methods: map[string]string{},
					}

					if d.Doc != nil {
						ti.doc = strings.TrimSpace(d.Doc.Text())
					} else if ts.Doc != nil {
						ti.doc = strings.TrimSpace(ts.Doc.Text())
					}

					if ts.Assign.IsValid() {
						ti.isAlias = true
						ti.aliasTarget = typeExprString(ts.Type)
					} else if st, ok := ts.Type.(*ast.StructType); ok {
						for _, field := range st.Fields.List {
							if len(field.Names) == 0 {
								ti.embeds = append(ti.embeds, typeExprString(field.Type))
								continue
							}
							fi := fieldInfo{
								name:     field.Names[0].Name,
								typeName: typeExprString(field.Type),
							}
							if field.Doc != nil {
								fi.doc = strings.TrimSpace(field.Doc.Text())
							}
							if field.Tag != nil {
								fi.tags = strings.TrimPrefix(strings.TrimSuffix(field.Tag.Value, "`"), "`")
							}
							ti.fields = append(ti.fields, fi)
						}
					} else {
						// Non-struct type definition (e.g., type Foo *string)
						ti.underlyingType = typeExprString(ts.Type)
					}

					types[ts.Name.Name] = ti
				}

			case *ast.FuncDecl:
				if d.Recv == nil || len(d.Recv.List) == 0 {
					continue
				}
				recvType := typeExprString(d.Recv.List[0].Type)
				methodName := d.Name.Name

				if d.Body != nil && len(d.Body.List) == 1 {
					if ret, ok := d.Body.List[0].(*ast.ReturnStmt); ok && len(ret.Results) == 1 {
						var val string
						switch r := ret.Results[0].(type) {
						case *ast.BasicLit:
							if r.Kind == token.STRING {
								val = strings.Trim(r.Value, "\"")
							}
						case *ast.Ident:
							// Captures true/false returns
							if r.Name == "true" || r.Name == "false" {
								val = r.Name
							}
						}
						if val != "" {
							if ti, ok := types[recvType]; ok {
								ti.methods[methodName] = val
							} else {
								ti := &typeInfo{name: recvType, methods: map[string]string{}}
								ti.methods[methodName] = val
								types[recvType] = ti
							}
						}
					}
				}
			}
		}
	}

	return types
}

func typeExprString(expr ast.Expr) string {
	switch e := expr.(type) {
	case *ast.Ident:
		return e.Name
	case *ast.StarExpr:
		return "*" + typeExprString(e.X)
	case *ast.ArrayType:
		return "[]" + typeExprString(e.Elt)
	case *ast.MapType:
		return "map[" + typeExprString(e.Key) + "]" + typeExprString(e.Value)
	case *ast.SelectorExpr:
		return typeExprString(e.X) + "." + e.Sel.Name
	case *ast.IndexExpr:
		// Generic with one type arg: Foo[T]
		return typeExprString(e.X) + "[" + typeExprString(e.Index) + "]"
	case *ast.IndexListExpr:
		// Generic with multiple type args: Foo[K, V]
		parts := make([]string, len(e.Indices))
		for i, idx := range e.Indices {
			parts[i] = typeExprString(idx)
		}
		return typeExprString(e.X) + "[" + strings.Join(parts, ", ") + "]"
	case *ast.InterfaceType:
		return "any"
	default:
		return "any"
	}
}

// ---------------------------------------------------------------------------
// Schema reconstruction
// ---------------------------------------------------------------------------

func buildTitleMap(types map[string]*typeInfo) map[string]string {
	m := map[string]string{}
	for goName, ti := range types {
		if title, ok := ti.methods["SchemaTitle"]; ok {
			m[goName] = title
		}
	}
	return m
}

func buildReverseTitleMap(titleMap map[string]string) map[string]string {
	m := map[string]string{}
	for goName, title := range titleMap {
		m[title] = goName
	}
	return m
}

func getTagValue(tags string, key string) string {
	return reflect.StructTag(tags).Get(key)
}

func reconstructSchema(
	goName string,
	types map[string]*typeInfo,
	titleMap map[string]string,
) map[string]any {
	ti, ok := types[goName]
	if !ok {
		return nil
	}

	title := titleMap[goName]
	result := map[string]any{"title": title}
	if ti.doc != "" {
		result["description"] = ti.doc
	}

	if ti.isAlias {
		target := ti.aliasTarget
		if strings.HasPrefix(target, "*") {
			inner := strings.TrimPrefix(target, "*")
			if innerTitle, ok := titleMap[inner]; ok {
				result["anyOf"] = []any{
					map[string]any{"$ref": innerTitle},
					map[string]any{"type": "null"},
				}
			}
		}
		return result
	}

	// Non-struct type definition (e.g., type Foo string, type Foo []Bar)
	if ti.underlyingType != "" {
		ut := ti.underlyingType
		isPtr := strings.HasPrefix(ut, "*")
		if isPtr {
			ut = strings.TrimPrefix(ut, "*")
		}
		inner := buildFieldTypeSchema(ut, types, titleMap)
		if isPtr {
			result["anyOf"] = []any{inner, map[string]any{"type": "null"}}
		} else {
			for k, v := range inner {
				result[k] = v
			}
		}
		return result
	}

	// Variant struct: all fields lack json tags (union type, not a regular struct)
	isVariant := len(ti.fields) > 0 && getTagValue(ti.fields[0].tags, "json") == ""

	if isVariant {
		return reconstructVariantSchema(ti, types, titleMap, result)
	}

	return reconstructStructSchema(ti, types, titleMap, result)
}

func reconstructStructSchema(
	ti *typeInfo,
	types map[string]*typeInfo,
	titleMap map[string]string,
	result map[string]any,
) map[string]any {
	result["type"] = "object"

	for _, embed := range ti.embeds {
		if embedTitle, ok := titleMap[embed]; ok {
			result["$ref"] = embedTitle
		}
	}

	properties := map[string]any{}
	for _, f := range ti.fields {
		jsonTag := getTagValue(f.tags, "json")
		if jsonTag == "" || jsonTag == "-" {
			continue
		}
		propName := strings.Split(jsonTag, ",")[0]
		isOmitempty := strings.Contains(jsonTag, "omitempty")

		propSchema := reconstructFieldSchema(f, isOmitempty, types, titleMap)
		if f.doc != "" {
			propSchema["description"] = f.doc
		}
		properties[propName] = propSchema
	}

	if len(properties) > 0 {
		result["properties"] = properties
	}

	if ap, ok := ti.methods["AdditionalProperties"]; ok {
		result["additionalProperties"] = ap == "true"
	}

	return result
}

func reconstructFieldSchema(
	f fieldInfo,
	isOmitempty bool,
	types map[string]*typeInfo,
	titleMap map[string]string,
) map[string]any {
	validateTag := getTagValue(f.tags, "validate")
	patternTag := getTagValue(f.tags, "pattern")
	defaultTag := getTagValue(f.tags, "default")

	typeName := f.typeName
	isPointer := strings.HasPrefix(typeName, "*")
	if isPointer {
		typeName = strings.TrimPrefix(typeName, "*")
	}

	inner := buildFieldTypeSchema(typeName, types, titleMap)

	if patternTag != "" {
		inner["pattern"] = patternTag
	}
	if defaultTag != "" {
		inner["default"] = parseDefaultValue(defaultTag)
	}
	addValidateConstraints(inner, validateTag)

	var result map[string]any
	if isPointer {
		result = map[string]any{
			"anyOf": []any{inner, map[string]any{"type": "null"}},
		}
	} else {
		result = inner
	}
	if isOmitempty {
		result["omitempty"] = true
	}
	return result
}

func buildFieldTypeSchema(
	typeName string,
	types map[string]*typeInfo,
	titleMap map[string]string,
) map[string]any {
	// Pointer types → nullable (anyOf with null)
	if strings.HasPrefix(typeName, "*") {
		inner := buildFieldTypeSchema(strings.TrimPrefix(typeName, "*"), types, titleMap)
		return map[string]any{
			"anyOf": []any{inner, map[string]any{"type": "null"}},
		}
	}

	if schemaTitle, ok := titleMap[typeName]; ok {
		return map[string]any{"$ref": schemaTitle}
	}

	// Inline variant struct (multi-variant anyOf generated for a field)
	if ti, ok := types[typeName]; ok && len(ti.fields) > 0 && getTagValue(ti.fields[0].tags, "json") == "" {
		var anyOf []any
		for _, f := range ti.fields {
			variant := reconstructVariant(f, types, titleMap)
			anyOf = append(anyOf, variant)
		}
		result := map[string]any{"anyOf": anyOf}
		if ti.doc != "" {
			result["description"] = ti.doc
		}
		return result
	}

	switch typeName {
	case "string":
		return map[string]any{"type": "string"}
	case "bool":
		return map[string]any{"type": "boolean"}
	case "float64":
		return map[string]any{"type": "number"}
	case "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64":
		return map[string]any{"type": "integer"}
	case "struct{}":
		return map[string]any{"type": "object"}
	case "JsonValue":
		return map[string]any{}
	case "time.Time":
		return map[string]any{"type": "string", "format": "date-time"}
	case "uuid.UUID":
		return map[string]any{"type": "string", "format": "uuid"}
	}

	if strings.HasPrefix(typeName, "[]") {
		elemType := strings.TrimPrefix(typeName, "[]")
		items := buildFieldTypeSchema(elemType, types, titleMap)
		return map[string]any{"type": "array", "items": items}
	}

	if strings.HasPrefix(typeName, "OrderedMap[string, ") {
		valType := typeName[len("OrderedMap[string, ") : len(typeName)-1]
		if valType == "JsonValue" {
			return map[string]any{"type": "object", "additionalProperties": true}
		}
		valSchema := buildFieldTypeSchema(valType, types, titleMap)
		return map[string]any{"type": "object", "additionalProperties": valSchema}
	}

	return map[string]any{}
}

func addValidateConstraints(schema map[string]any, validateTag string) {
	if validateTag == "" {
		return
	}
	parts := strings.Split(validateTag, ",")

	// Find dive index — constraints before dive apply to the field,
	// constraints after dive apply to array items.
	diveIdx := -1
	for i, part := range parts {
		if part == "dive" {
			diveIdx = i
			break
		}
	}

	// Field-level constraints (before dive, or all if no dive)
	fieldParts := parts
	if diveIdx >= 0 {
		fieldParts = parts[:diveIdx]
	}
	for _, part := range fieldParts {
		if strings.HasPrefix(part, "oneof=") {
			vals := strings.Split(strings.TrimPrefix(part, "oneof="), " ")
			enumVals := make([]any, len(vals))
			for i, v := range vals {
				enumVals[i] = v
			}
			schema["enum"] = enumVals
		}
		if strings.HasPrefix(part, "min=") {
			schema[boundKeyword(schema, "min")] = json.Number(strings.TrimPrefix(part, "min="))
		}
		if strings.HasPrefix(part, "max=") {
			schema[boundKeyword(schema, "max")] = json.Number(strings.TrimPrefix(part, "max="))
		}
	}

	// Item/value-level constraints (after dive) — each "dive" navigates one
	// level deeper into items/additionalProperties before applying constraints.
	if diveIdx >= 0 && diveIdx+1 < len(parts) {
		target := schema
		for _, part := range parts[diveIdx:] {
			if part == "dive" {
				if items, ok := target["items"].(map[string]any); ok {
					target = items
				} else if ap, ok := target["additionalProperties"].(map[string]any); ok {
					target = ap
				} else {
					break
				}
				continue
			}
			if strings.HasPrefix(part, "min=") {
				target[boundKeyword(target, "min")] = json.Number(strings.TrimPrefix(part, "min="))
			}
			if strings.HasPrefix(part, "max=") {
				target[boundKeyword(target, "max")] = json.Number(strings.TrimPrefix(part, "max="))
			}
		}
	}
}

// The JSON Schema keyword a validate `min=`/`max=` maps back to,
// derived from the target's type — the validator's min/max mean
// LENGTH on a slice (minItems/maxItems) and VALUE on a number
// (minimum/maximum). Never mixed: the tag carries the number, the Go
// type picks the keyword (the Go-roundtrip reconstruction rule).
func boundKeyword(target map[string]any, side string) string {
	if target["type"] == "array" {
		if side == "min" {
			return "minItems"
		}
		return "maxItems"
	}
	if side == "min" {
		return "minimum"
	}
	return "maximum"
}


func parseDefaultValue(s string) any {
	if s == "true" {
		return true
	}
	if s == "false" {
		return false
	}
	if s == "null" {
		return nil
	}
	var n json.Number
	if err := json.Unmarshal([]byte(s), &n); err == nil {
		return n
	}
	// Try to decode as a JSON array or object so an empty Vec default
	// (`default:"[]"`) round-trips as an actual JSON array, not the
	// string "[]".
	if (strings.HasPrefix(s, "[") && strings.HasSuffix(s, "]")) ||
		(strings.HasPrefix(s, "{") && strings.HasSuffix(s, "}")) {
		var v any
		if err := json.Unmarshal([]byte(s), &v); err == nil {
			return v
		}
	}
	return s
}

func reconstructVariantSchema(
	ti *typeInfo,
	types map[string]*typeInfo,
	titleMap map[string]string,
	result map[string]any,
) map[string]any {
	// Detect flat root enum: no variant type has SchemaVariantTitle
	if isRootEnum(ti, types, titleMap) {
		var allEnumVals []any
		for _, f := range ti.fields {
			validateTag := getTagValue(f.tags, "validate")
			for _, part := range strings.Split(validateTag, ",") {
				if strings.HasPrefix(part, "oneof=") {
					for _, v := range strings.Split(strings.TrimPrefix(part, "oneof="), " ") {
						allEnumVals = append(allEnumVals, v)
					}
				}
			}
		}
		result["type"] = "string"
		result["enum"] = allEnumVals
		return result
	}

	// anyOf with titled variants
	var anyOf []any
	for _, f := range ti.fields {
		variant := reconstructVariant(f, types, titleMap)
		anyOf = append(anyOf, variant)
	}
	result["anyOf"] = anyOf
	return result
}

// isRootEnum checks if a variant struct represents a flat root enum
// (no SchemaVariantTitle on any variant type) vs an anyOf with titled variants.
func isRootEnum(ti *typeInfo, types map[string]*typeInfo, titleMap map[string]string) bool {
	for _, f := range ti.fields {
		typeName := strings.TrimPrefix(f.typeName, "*")
		// If the variant type has SchemaVariantTitle → anyOf
		if subTi, ok := types[typeName]; ok {
			if _, has := subTi.methods["SchemaVariantTitle"]; has {
				return false
			}
		}
		// If the variant type has SchemaTitle → it's a $ref variant, not enum
		if _, ok := titleMap[typeName]; ok {
			return false
		}
	}
	return true
}

// addInlineStructProps adds properties and additionalProperties to a variant
// schema from an inline sub-struct's fields and methods.
func addInlineStructProps(variant map[string]any, subTi *typeInfo, types map[string]*typeInfo, titleMap map[string]string) {
	props := map[string]any{}
	for _, sf := range subTi.fields {
		jsonTag := getTagValue(sf.tags, "json")
		if jsonTag == "" || jsonTag == "-" {
			continue
		}
		propName := strings.Split(jsonTag, ",")[0]
		propSchema := reconstructFieldSchema(sf, strings.Contains(jsonTag, "omitempty"), types, titleMap)
		if sf.doc != "" {
			propSchema["description"] = sf.doc
		}
		props[propName] = propSchema
	}
	if len(props) > 0 {
		variant["properties"] = props
	}
	if ap, ok := subTi.methods["AdditionalProperties"]; ok {
		variant["additionalProperties"] = ap == "true"
	}
}

func reconstructVariant(
	f fieldInfo,
	types map[string]*typeInfo,
	titleMap map[string]string,
) map[string]any {
	typeName := f.typeName
	if strings.HasPrefix(typeName, "*") {
		typeName = strings.TrimPrefix(typeName, "*")
	}

	// Get variant title. Priority:
	//   1. `variantTitle:"X"` struct tag — generator stamps this on
	//      $ref variants whose field name differs from the title
	//      (e.g. `MCP` field → `Mcp` title).
	//   2. SchemaVariantTitle() method — generator emits this for
	//      inline + primitive variants.
	//   3. Field name — fallback.
	variantTitle := f.name
	if subTi, ok := types[typeName]; ok {
		if svt, ok := subTi.methods["SchemaVariantTitle"]; ok {
			variantTitle = svt
		}
	}
	if vt := getTagValue(f.tags, "variantTitle"); vt != "" {
		variantTitle = vt
	}

	// Build the inner schema (without title/description)
	inner := reconstructVariantInner(f, typeName, types, titleMap)

	// Assemble the variant with title, description, and inner schema
	variant := map[string]any{"title": variantTitle}
	if f.doc != "" {
		variant["description"] = f.doc
	}
	for k, v := range inner {
		variant[k] = v
	}
	return variant
}

func reconstructVariantInner(
	f fieldInfo,
	typeName string,
	types map[string]*typeInfo,
	titleMap map[string]string,
) map[string]any {
	// `outerObject:"true"` tag is set by install_go.go whenever schemars
	// stamped `type: "object"` next to the variant's `$ref` (the outer
	// schema is a struct that flattens an untagged enum, or an
	// internally-tagged enum). Re-emit `"type": "object"` alongside the
	// `$ref` so the roundtrip matches the on-disk shape.
	outerObject := getTagValue(f.tags, "outerObject") == "true"

	if subTi, ok := types[typeName]; ok && !subTi.isAlias {
		// If the sub-type has its own SchemaTitle, it's a standalone type → $ref
		if subTitle, ok := titleMap[typeName]; ok {
			result := map[string]any{}
			if outerObject {
				result["type"] = "object"
			}
			result["$ref"] = subTitle
			return result
		}

		// Type definition with underlyingType (primitive variant, e.g., type FooBar string)
		if subTi.underlyingType != "" {
			inner := buildFieldTypeSchema(subTi.underlyingType, types, titleMap)
			validateTag := getTagValue(f.tags, "validate")
			addValidateConstraints(inner, validateTag)
			return inner
		}

		// Inline sub-struct with embedded type → adjacently-tagged ($ref + properties)
		if len(subTi.embeds) > 0 {
			result := map[string]any{"type": "object"}
			embedType := subTi.embeds[0]
			if embedTitle, ok := titleMap[embedType]; ok {
				result["$ref"] = embedTitle
			}
			addInlineStructProps(result, subTi, types, titleMap)
			return result
		}

		// Sum-type variant: the field is a struct whose own fields are
		// each tagged variants (pointers to types carrying
		// SchemaVariantTitle, no json tag — they're untagged-union
		// alternatives, not properties). Emit anyOf.
		isSumType := len(subTi.fields) > 0
		for _, sf := range subTi.fields {
			jsonTag := getTagValue(sf.tags, "json")
			if jsonTag != "" && jsonTag != "-" {
				isSumType = false
				break
			}
			subType := strings.TrimPrefix(sf.typeName, "*")
			if subTi2, ok := types[subType]; ok {
				if _, has := subTi2.methods["SchemaVariantTitle"]; !has {
					isSumType = false
					break
				}
			} else if _, ok := titleMap[subType]; !ok {
				isSumType = false
				break
			}
		}
		if isSumType {
			result := map[string]any{}
			anyOf := make([]any, 0, len(subTi.fields))
			for _, sf := range subTi.fields {
				anyOf = append(anyOf, reconstructVariant(sf, types, titleMap))
			}
			result["anyOf"] = anyOf
			return result
		}

		// Inline object without embedding
		result := map[string]any{"type": "object"}
		addInlineStructProps(result, subTi, types, titleMap)
		return result
	}

	// Known schema type → $ref
	if refTitle, ok := titleMap[typeName]; ok {
		result := map[string]any{}
		if outerObject {
			result["type"] = "object"
		}
		result["$ref"] = refTitle
		return result
	}

	// Primitive variant (string with enum)
	validateTag := getTagValue(f.tags, "validate")
	inner := buildFieldTypeSchema(typeName, types, titleMap)
	addValidateConstraints(inner, validateTag)
	return inner
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

func TestRoundtrip(t *testing.T) {
	types := parseSourceDir(t)
	titleMap := buildTitleMap(types)
	reverseMap := buildReverseTitleMap(titleMap)

	for _, title := range AllTitlesSorted {
		goName, ok := reverseMap[title]
		if !ok {
			t.Run(title, func(t *testing.T) {
				t.Fatalf("no Go type with SchemaTitle() = %q", title)
			})
			continue
		}

		t.Run(title, func(t *testing.T) {
			schema := reconstructSchema(goName, types, titleMap)
			if schema == nil {
				t.Fatalf("failed to reconstruct schema for %s", goName)
			}
			AssertSchemaMatches(t, title, schema)
		})
	}
}
