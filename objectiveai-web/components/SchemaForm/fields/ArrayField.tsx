"use client";

import type { FieldProps, ArrayInputSchema, InputValue } from "../types";
import { getErrorsForPath, getErrorMessage } from "../validation";
import { joinPath, getDefaultValue } from "../utils";
import SchemaField from "../SchemaField";

interface ArrayFieldProps extends FieldProps<InputValue[]> {
  schema: ArrayInputSchema;
}

export default function ArrayField({
  schema,
  value,
  onChange,
  path,
  errors,
  disabled,
  isMobile,
  depth = 0,
}: ArrayFieldProps) {
  const items = Array.isArray(value) ? value : [];
  const canAdd = schema.maxItems == null || items.length < schema.maxItems;
  const canRemove = items.length > (schema.minItems ?? 0);

  const errorMessage = getErrorMessage(errors, path);

  const addItem = () => {
    const defaultVal = getDefaultValue(schema.items);
    onChange([...items, defaultVal]);
  };

  const removeItem = (index: number) => {
    onChange(items.filter((_, i) => i !== index));
  };

  const updateItem = (index: number, newVal: InputValue) => {
    const updated = [...items];
    updated[index] = newVal;
    onChange(updated);
  };

  // Visual styling based on depth
  const bgColors = [
    "transparent",
    "rgba(107, 92, 255, 0.02)",
    "rgba(107, 92, 255, 0.04)",
    "rgba(107, 92, 255, 0.06)",
  ];
  const bg = bgColors[(depth + 1) % bgColors.length];

  return (
    <div className="schemaFieldWrapper">
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "8px",
        }}
      >
        {items.map((item, index) => {
          const itemPath = joinPath(path, index);
          const itemErrors = getErrorsForPath(errors, itemPath);

          return (
            <div
              key={index}
              style={{
                display: "flex",
                gap: "8px",
                alignItems: "flex-start",
                padding: "12px",
                background: bg,
                borderRadius: "8px",
                border: "1px solid var(--border)",
              }}
            >
              {/* Index indicator */}
              <span
                style={{
                  fontSize: "12px",
                  fontWeight: 500,
                  color: "var(--text-muted)",
                  minWidth: "24px",
                  paddingTop: "10px",
                  fontFamily: "var(--font-mono, monospace)",
                }}
              >
                {index + 1}.
              </span>

              {/* Item content */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <SchemaField
                  schema={schema.items}
                  value={item}
                  onChange={(newVal) => updateItem(index, newVal)}
                  path={itemPath}
                  errors={itemErrors}
                  disabled={disabled}
                  isMobile={isMobile}
                  depth={depth + 1}
                />
              </div>

              {/* Remove button */}
              {canRemove && (
                <button
                  type="button"
                  onClick={() => removeItem(index)}
                  disabled={disabled}
                  className="iconBtn iconBtn-sm"
                  style={{
                    color: "var(--color-error)",
                    marginTop: "6px",
                  }}
                  aria-label="Remove item"
                >
                  <CloseIcon />
                </button>
              )}
            </div>
          );
        })}

        {/* Add button */}
        {canAdd && (
          <button
            type="button"
            onClick={addItem}
            disabled={disabled}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "6px",
              padding: isMobile ? "12px" : "10px",
              border: "1px dashed var(--border)",
              borderRadius: "8px",
              background: "transparent",
              color: "var(--text-muted)",
              cursor: disabled ? "not-allowed" : "pointer",
              fontSize: "13px",
              transition: "border-color 0.2s, color 0.2s",
            }}
            onMouseEnter={(e) => {
              if (!disabled) {
                e.currentTarget.style.borderColor = "var(--accent)";
                e.currentTarget.style.color = "var(--accent)";
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = "var(--border)";
              e.currentTarget.style.color = "var(--text-muted)";
            }}
          >
            <PlusIcon />
            <span>Add item</span>
          </button>
        )}
      </div>

      {errorMessage && <span className="fieldError">{errorMessage}</span>}

      {/* Show min/max constraints */}
      {(schema.minItems != null || schema.maxItems != null) && (
        <span className="fieldHint">
          {schema.minItems != null && `Min: ${schema.minItems}`}
          {schema.minItems != null && schema.maxItems != null && " · "}
          {schema.maxItems != null && `Max: ${schema.maxItems}`}
        </span>
      )}
    </div>
  );
}

function CloseIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}
