"use client";

import { useState, useCallback } from "react";
import styles from "./SchemaForm.module.css";

/* ── Types ── */

interface SchemaProperty {
  type?: string;
  description?: string;
  enum?: string[];
  items?: { type?: string; anyOf?: Array<{ type?: string }> };
  minItems?: number;
  anyOf?: Array<{ type?: string }>;
}

interface InputSchema {
  type?: string;
  properties?: Record<string, SchemaProperty>;
  required?: string[];
  description?: string;
}

export interface SchemaFormProps {
  schema: InputSchema;
  onSubmit: (values: Record<string, unknown>) => void;
  disabled?: boolean;
}

/* ── Component ── */

export function SchemaForm({ schema, onSubmit, disabled }: SchemaFormProps) {
  const properties = schema.properties ?? {};
  const required = schema.required ?? [];
  const keys = Object.keys(properties);

  const [values, setValues] = useState<Record<string, unknown>>(() => {
    const init: Record<string, unknown> = {};
    for (const [key, prop] of Object.entries(properties)) {
      init[key] = getDefault(prop);
    }
    return init;
  });

  const update = useCallback((key: string, val: unknown) => {
    setValues((prev) => ({ ...prev, [key]: val }));
  }, []);

  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(values);
  }, [values, onSubmit]);

  if (keys.length === 0) return null;

  return (
    <form className={styles.form} onSubmit={handleSubmit}>
      {keys.map((key) => (
        <Field
          key={key}
          name={key}
          prop={properties[key]}
          required={required.includes(key)}
          value={values[key]}
          onChange={(v) => update(key, v)}
          disabled={disabled}
        />
      ))}
      <div className={styles.actions}>
        <button type="submit" className={styles.submit} disabled={disabled}>
          execute
        </button>
      </div>
    </form>
  );
}

/* ── Field renderer ── */

function Field({
  name,
  prop,
  required,
  value,
  onChange,
  disabled,
}: {
  name: string;
  prop: SchemaProperty;
  required: boolean;
  value: unknown;
  onChange: (v: unknown) => void;
  disabled?: boolean;
}) {
  const resolvedType = resolveType(prop);

  return (
    <div className={styles.field}>
      <label className={styles.label}>
        <span className={styles.labelName}>{name}</span>
        <span className={styles.labelType}>{resolvedType}</span>
        {required && <span className={styles.labelRequired}>required</span>}
        {prop.description && <span className={styles.labelDesc}>{prop.description}</span>}
      </label>
      {renderInput(resolvedType, prop, value, onChange, disabled)}
    </div>
  );
}

/* ── Input renderers by type ── */

function renderInput(
  type: string,
  prop: SchemaProperty,
  value: unknown,
  onChange: (v: unknown) => void,
  disabled?: boolean,
): React.ReactNode {
  // Enum → select
  if (prop.enum && prop.enum.length > 0) {
    return (
      <select
        className={styles.select}
        value={String(value ?? "")}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      >
        <option value="">select...</option>
        {prop.enum.map((opt) => (
          <option key={opt} value={opt}>{opt}</option>
        ))}
      </select>
    );
  }

  switch (type) {
    case "string":
      return (
        <input
          className={styles.input}
          type="text"
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
          placeholder={prop.description ?? ""}
          disabled={disabled}
        />
      );

    case "number":
    case "integer":
      return (
        <input
          className={styles.input}
          type="number"
          value={value === null || value === undefined ? "" : String(value)}
          onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))}
          placeholder={prop.description ?? "0"}
          step={type === "integer" ? 1 : "any"}
          disabled={disabled}
        />
      );

    case "boolean":
      return (
        <label className={styles.checkbox}>
          <input
            className={styles.checkboxInput}
            type="checkbox"
            checked={Boolean(value)}
            onChange={(e) => onChange(e.target.checked)}
            disabled={disabled}
          />
          {value ? "true" : "false"}
        </label>
      );

    case "array":
      return (
        <ArrayField
          prop={prop}
          value={Array.isArray(value) ? value : []}
          onChange={onChange}
          disabled={disabled}
        />
      );

    // Complex types (object, image, audio, video, file) → JSON textarea
    default:
      return (
        <textarea
          className={styles.textarea}
          value={typeof value === "string" ? value : JSON.stringify(value, null, 2)}
          onChange={(e) => {
            try { onChange(JSON.parse(e.target.value)); } catch { onChange(e.target.value); }
          }}
          placeholder={`${type} (JSON)`}
          disabled={disabled}
        />
      );
  }
}

/* ── Array field with add/remove ── */

function ArrayField({
  prop,
  value,
  onChange,
  disabled,
}: {
  prop: SchemaProperty;
  value: unknown[];
  onChange: (v: unknown[]) => void;
  disabled?: boolean;
}) {
  const itemType = prop.items?.type ?? prop.items?.anyOf?.[0]?.type ?? "string";

  const addItem = () => {
    onChange([...value, itemType === "number" ? 0 : ""]);
  };

  const removeItem = (idx: number) => {
    onChange(value.filter((_, i) => i !== idx));
  };

  const updateItem = (idx: number, val: unknown) => {
    const next = [...value];
    next[idx] = val;
    onChange(next);
  };

  return (
    <div className={styles.arrayItems}>
      {value.map((item, i) => (
        <div key={i} className={styles.arrayRow}>
          <input
            className={styles.input}
            type={itemType === "number" ? "number" : "text"}
            value={String(item ?? "")}
            onChange={(e) => updateItem(i, itemType === "number" ? Number(e.target.value) : e.target.value)}
            placeholder={`item ${i}`}
            disabled={disabled}
          />
          <button
            type="button"
            className={styles.arrayRemove}
            onClick={() => removeItem(i)}
            disabled={disabled}
          >
            ×
          </button>
        </div>
      ))}
      <button
        type="button"
        className={styles.arrayAdd}
        onClick={addItem}
        disabled={disabled}
      >
        + add item
      </button>
    </div>
  );
}

/* ── Helpers ── */

function resolveType(prop: SchemaProperty): string {
  if (prop.anyOf) {
    return prop.anyOf.map((v) => v.type ?? "unknown").join(" | ");
  }
  return prop.type ?? "object";
}

function getDefault(prop: SchemaProperty): unknown {
  const type = resolveType(prop);
  if (prop.enum?.length) return "";
  switch (type) {
    case "string": return "";
    case "number":
    case "integer": return null;
    case "boolean": return false;
    case "array": {
      const min = prop.minItems ?? 0;
      const itemType = prop.items?.type ?? prop.items?.anyOf?.[0]?.type ?? "string";
      return Array.from({ length: min }, () => itemType === "number" ? 0 : "");
    }
    default: return null;
  }
}
