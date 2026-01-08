import { Provider } from "@/provider";
import { ReactElement } from "react";
import cn from "classnames";
import { SharedHeader } from "../SharedHeader";
import { SharedFooter } from "../SharedFooter";
import { MarkdownContent } from "../Markdown";
import { Sidebar } from "./Sidebar";
import z from "zod";

export function EndpointDocs({
  session,
  requestHeaders,
  requestPath,
  requestQuery,
  requestBody,
  responseBody,
  responseBodyStreaming,
}: {
  session?: Provider.TokenSession;
  requestHeaders?: z.ZodType;
  requestPath?: z.ZodType;
  requestQuery?: z.ZodType;
  requestBody?: z.ZodType;
  responseBody?: z.ZodType;
  responseBodyStreaming?: z.ZodType;
}): ReactElement {
  return (
    <main className={cn("h-[100dvh]", "w-[100dvw]", "flex", "flex-col")}>
      <SharedHeader session={session} />
      <div className={cn("flex-grow", "flex", "overflow-hidden")}>
        <Sidebar />
        <div
          className={cn(
            "flex-grow",
            "flex",
            "flex-col",
            "overflow-auto",
            "basis-0",
            "px-4"
          )}
        >
          <div
            className={cn(
              "w-[calc(var(--spacing)*192)]",
              "max-w-full",
              "space-y-8",
              "mx-auto",
              "my-8"
            )}
          >
            {requestHeaders && (
              <Section title="Request Headers" schema={requestHeaders} />
            )}
            {requestPath && (
              <Section title="Request Path" schema={requestPath} />
            )}
            {requestQuery && (
              <Section title="Request Query" schema={requestQuery} />
            )}
            {requestBody && (
              <Section title="Request Body" schema={requestBody} />
            )}
            {responseBody && (
              <Section title="Response Body" schema={responseBody} />
            )}
            {responseBodyStreaming && (
              <Section
                title="Response Body (Streaming)"
                schema={responseBodyStreaming}
              />
            )}
          </div>
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}

function Section({
  title,
  schema,
  className,
}: {
  title: string;
  schema: z.ZodType;
  className?: string;
}): ReactElement {
  return (
    <div className={cn(className)}>
      <h3 className={cn("text-2xl", "sm:text-3xl", "font-medium")}>{title}</h3>
      <div
        className={cn(
          "space-y-4",
          "border-t",
          "border-muted-secondary",
          "pt-4",
          "mt-4"
        )}
      >
        <Property schema={schema} expanded />
      </div>
    </div>
  );
}

export function Property({
  name,
  schema,
  description,
  title,
  optional = false,
  nullable = false,
  expanded = false,
  className,
}: {
  name?: string;
  schema: z.ZodType;
  description?: string;
  title?: string;
  optional?: boolean;
  nullable?: boolean;
  expanded?: boolean;
  className?: string;
}): ReactElement {
  // track optional and nullable and description
  const params = { description, title, optional, nullable };

  // unwrap schema
  schema = unwrapSchema(schema, params);

  // get variants or properties
  const properties: {
    name?: string;
    schema: z.ZodType;
    description?: string;
    optional: boolean;
    nullable: boolean;
  }[] = [];
  getSchemaChildren(schema, properties, params);

  if (expanded) {
    return (
      <div className={cn("space-y-4", className)}>
        {properties.map(
          (
            {
              name: propertyName,
              schema: propertySchema,
              description: propertyDescription,
              optional: propertyOptional,
              nullable: propertyNullable,
            },
            index
          ) => (
            <Property
              key={index}
              name={propertyName}
              schema={propertySchema}
              description={propertyDescription}
              optional={propertyOptional}
              nullable={propertyNullable}
            />
          )
        )}
      </div>
    );
  } else {
    return (
      <div className={cn(className)}>
        <div className={cn("space-x-2")}>
          {(name || params.title) && (
            <span
              className={cn(
                "text-base",
                "sm:text-lg",
                "font-mono",
                "font-bold"
              )}
            >
              {name || params.title}
            </span>
          )}
          {schema.def.type === "literal" ? (
            <span className={cn("bg-background-code", "px-1", "py-0.5")}>
              {`"${(schema as z.ZodLiteral).value}"`}
            </span>
          ) : (
            schema.def.type !== "lazy" && (
              <span
                className={cn(
                  "font-mono",
                  "bg-background-code",
                  "px-1",
                  "py-0.5"
                )}
              >
                {schema.def.type}
              </span>
            )
          )}
          {"minValue" in schema &&
            schema.minValue !== null &&
            schema.minValue !== -Infinity && (
              <span
                className={cn(
                  "font-mono",
                  "bg-background-code",
                  "px-1",
                  "py-0.5"
                )}
              >
                min: {`${schema.minValue}`}
              </span>
            )}
          {"maxValue" in schema &&
            schema.maxValue !== null &&
            schema.maxValue !== Infinity && (
              <span
                className={cn(
                  "font-mono",
                  "bg-background-code",
                  "px-1",
                  "py-0.5"
                )}
              >
                max: {`${schema.maxValue}`}
              </span>
            )}
          {name &&
            (params.optional || params.nullable ? (
              <span className={cn("text-muted-primary")}>optional</span>
            ) : (
              <span className={cn("text-red-500")}>*required</span>
            ))}
        </div>
        {params.description && (
          <div className={cn("text-secondary")}>
            <MarkdownContent content={params.description} />
          </div>
        )}
        {properties.length > 0 && (
          <details>
            <summary
              className={cn("list-item", "list-outside", "cursor-pointer")}
            >
              {schema.def.type === "object" && "Properties"}
              {schema.def.type === "record" && "Values"}
              {schema.def.type === "union" && "Variants"}
              {schema.def.type === "enum" && "Variants"}
              {schema.def.type === "array" && "Items"}
            </summary>
            <div className={cn("ml-4", "mt-4", "space-y-4")}>
              {properties.map(
                (
                  {
                    name: propertyName,
                    schema: propertySchema,
                    description: propertyDescription,
                    optional: propertyOptional,
                    nullable: propertyNullable,
                  },
                  index
                ) => (
                  <Property
                    key={index}
                    name={propertyName}
                    schema={propertySchema}
                    description={propertyDescription}
                    optional={propertyOptional}
                    nullable={propertyNullable}
                  />
                )
              )}
            </div>
          </details>
        )}
      </div>
    );
  }
}

interface SchemaParams {
  description?: string;
  title?: string;
  optional: boolean;
  nullable: boolean;
}

function unwrapSchema(schema: z.ZodType, params: SchemaParams): z.ZodType {
  // extract description
  if (schema.description && !params.description) {
    params.description = schema.description;
  }
  // don't unwrap lazy if recursive is true
  let meta = schema.meta();
  // extract title
  if (meta?.title && !params.title) {
    params.title = meta.title;
  }
  // unwrap optional/nullable/lazy
  while (
    (schema.def.type === "optional" ||
      schema.def.type === "nullable" ||
      schema.def.type === "lazy") &&
    meta?.recursive !== true
  ) {
    // flatten schema
    if (schema.def.type === "optional") {
      params.optional = true;
      schema = (schema as z.ZodOptional<z.ZodType>).unwrap();
    } else if (schema.def.type === "nullable") {
      params.nullable = true;
      schema = (schema as z.ZodNullable<z.ZodType>).unwrap();
    } else if (schema.def.type === "lazy") {
      schema = (schema as z.ZodLazy<z.ZodType>).def.getter();
    }
    // extract description
    if (schema.description && !params.description) {
      params.description = schema.description;
    }
    // replace meta
    meta = schema.meta();
    // extract title
    if (meta?.title && !params.title) {
      params.title = meta.title;
    }
  }
  // return schema
  return schema;
}

interface SchemaProperty {
  name?: string;
  schema: z.ZodType;
  description?: string;
  title?: string;
  optional: boolean;
  nullable: boolean;
}

function getSchemaChildren(
  schema: z.ZodType,
  properties: SchemaProperty[],
  optional: { optional: boolean; nullable: boolean }
) {
  const meta = schema.meta();
  if (meta?.recursive === true) {
    return;
  } else if (schema.def.type === "object") {
    for (const [key, value] of Object.entries((schema as z.ZodObject).shape)) {
      const propertyMeta = value.meta();
      if (propertyMeta?.recursive === true) {
        properties.push({
          name: key,
          schema: value,
          description: value.description,
          title: propertyMeta?.title,
          optional: false,
          nullable: false,
        });
      } else {
        const propertyParams = {
          optional: false,
          nullable: false,
          description: undefined,
          title: undefined,
        };
        const property = unwrapSchema(value, propertyParams);
        properties.push({
          name: key,
          schema: property,
          description: propertyParams.description,
          title: propertyParams.title,
          optional: propertyParams.optional,
          nullable: propertyParams.nullable,
        });
      }
    }
  } else if (schema.def.type === "record") {
    const value = (schema as z.ZodRecord<z.core.$ZodRecordKey, z.ZodType>)
      .valueType;
    const propertyMeta = value.meta();
    if (propertyMeta?.recursive === true) {
      properties.push({
        schema: value,
        description: value.description,
        title: propertyMeta?.title,
        optional: false,
        nullable: false,
      });
    } else {
      const propertyParams = {
        optional: false,
        nullable: false,
        description: undefined,
        title: undefined,
      };
      const property = unwrapSchema(
        (schema as z.ZodRecord<z.core.$ZodRecordKey, z.ZodType>).valueType,
        propertyParams
      );
      if (flattenable(property)) {
        getSchemaChildren(property, properties, propertyParams);
      } else {
        properties.push({
          schema: property,
          description: propertyParams.description,
          title: propertyParams.title,
          optional: false,
          nullable: false,
        });
      }
      if (propertyParams.optional || propertyParams.nullable) {
        let undefinedProperty = false;
        let nullProperty = false;
        for (const property of properties) {
          undefinedProperty ||= property.schema.def.type === "undefined";
          nullProperty ||= property.schema.def.type === "null";
        }
        if (propertyParams.optional && !undefinedProperty) {
          properties.push({
            schema: z.undefined(),
            optional: false,
            nullable: false,
          });
        }
        if (propertyParams.nullable && !nullProperty) {
          properties.push({
            schema: z.null(),
            optional: false,
            nullable: false,
          });
        }
      }
    }
  } else if (schema.def.type === "union") {
    for (const option of (schema as z.ZodUnion<z.ZodType[]>).options) {
      const propertyMeta = option.meta();
      if (propertyMeta?.recursive === true) {
        properties.push({
          schema: option,
          description: option.description,
          title: propertyMeta?.title,
          optional: false,
          nullable: false,
        });
      } else {
        const propertyParams = {
          optional: false,
          nullable: false,
          description: undefined,
          title: undefined,
        };
        const property = unwrapSchema(option, propertyParams);
        optional.optional ||= propertyParams.optional;
        optional.nullable ||= propertyParams.nullable;
        if (flattenable(property)) {
          getSchemaChildren(property, properties, optional);
        } else {
          properties.push({
            schema: property,
            description: propertyParams.description,
            title: propertyParams.title,
            optional: false,
            nullable: false,
          });
        }
      }
    }
  } else if (schema.def.type === "enum") {
    for (const option of (schema as z.ZodEnum).options) {
      properties.push({
        schema: z.literal(option),
        optional: false,
        nullable: false,
      });
    }
  } else if (schema.def.type === "array") {
    const element = (schema as z.ZodArray<z.ZodType>).element;
    const propertyMeta = element.meta();
    if (propertyMeta?.recursive === true) {
      properties.push({
        schema: element,
        description: element.description,
        title: propertyMeta?.title,
        optional: false,
        nullable: false,
      });
    } else {
      const propertyParams = {
        optional: false,
        nullable: false,
        description: undefined,
        title: undefined,
      };
      const property = unwrapSchema(
        (schema as z.ZodArray<z.ZodType>).element,
        propertyParams
      );
      if (flattenable(property)) {
        getSchemaChildren(property, properties, propertyParams);
      } else {
        properties.push({
          schema: property,
          description: propertyParams.description,
          title: propertyParams.title,
          optional: false,
          nullable: false,
        });
      }
      if (propertyParams.optional || propertyParams.nullable) {
        let undefinedProperty = false;
        let nullProperty = false;
        for (const property of properties) {
          undefinedProperty ||= property.schema.def.type === "undefined";
          nullProperty ||= property.schema.def.type === "null";
        }
        if (propertyParams.optional && !undefinedProperty) {
          properties.push({
            schema: z.undefined(),
            optional: false,
            nullable: false,
          });
        }
        if (propertyParams.nullable && !nullProperty) {
          properties.push({
            schema: z.null(),
            optional: false,
            nullable: false,
          });
        }
      }
    }
  }
}

function flattenable(schema: z.ZodType): boolean {
  return schema.def.type === "union" || schema.def.type === "enum";
}
