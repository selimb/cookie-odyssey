import type { Controller } from "@hotwired/stimulus";

type TargetDef = keyof HTMLElementTagNameMap | "element";
type TargetDefs = Record<string, TargetDef>;

type InferTargetDef<T extends TargetDef> = T extends keyof HTMLElementTagNameMap
  ? HTMLElementTagNameMap[T]
  : HTMLElement;

// eslint-disable-next-line @typescript-eslint/explicit-function-return-type -- Type inferrence is easier here.
export function defineTargets<T extends TargetDefs>(targetDefs: T) {
  return {
    targets: Object.keys(targetDefs),
    getTarget<K extends keyof T>(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any -- This is the way.
      this: Record<string, any>,
      name: K,
    ): InferTargetDef<T[K]> {
      const key = `${String(name)}Target`;
      // NOTE: Stimulus automatically throws if the target is not found.
      return this[key] as never;
    },
  };
}

// This isn't publicly exported, and I don't like importing from `dist`.
type StimulusValueDefs = (typeof Controller)["values"];
type ValueDef = "string" | "number" | "boolean";
type ValueDefs = Record<string, ValueDef>;

type InferValueDef<T extends ValueDef> = T extends "string"
  ? string
  : T extends "number"
    ? number
    : T extends "boolean"
      ? boolean
      : never;

const valueDefMap: Record<
  ValueDef,
  StimulusValueDefs[keyof StimulusValueDefs]
> = {
  string: String,
  number: Number,
  boolean: Boolean,
};

// eslint-disable-next-line @typescript-eslint/explicit-function-return-type -- Type inferrence is easier here.
export function defineValues<T extends ValueDefs>(valueDefs: T) {
  const stimulusValueDefs: StimulusValueDefs = Object.fromEntries(
    Object.entries(valueDefs).map(([key, valueDef]) => {
      return [key, valueDefMap[valueDef]];
    }),
  );

  return {
    values: stimulusValueDefs,
    getValue<K extends keyof T>(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any -- This is the way.
      this: Record<string, any>,
      name: K,
    ): InferValueDef<T[K]> {
      const existentialKey = `has${capitalize(String(name))}Value`;
      const exists = this[existentialKey] as boolean;
      if (!exists) throw new Error(`Missing value: ${String(name)}`);

      const getterKey = `${String(name)}Value`;
      const value = this[getterKey] as never;
      return value;
    },
  };
}

function capitalize(key: string): string {
  return key.charAt(0).toUpperCase() + key.slice(1);
}
