/**
 * Strongly typed Stimulus helpers.
 * Inspired by https://github.com/ajaishankar/stimulus-typescript/tree/main
 */
import { Controller } from "@hotwired/stimulus";

type TargetDef = keyof HTMLElementTagNameMap | "element";
type TargetDefs = Record<string, TargetDef>;

type InferTargetDef<T extends TargetDef> = T extends keyof HTMLElementTagNameMap
  ? HTMLElementTagNameMap[T]
  : HTMLElement;

// This isn't publicly exported, and I don't like importing from `dist`.
type StimulusValueDefs = (typeof Controller)["values"];
type ValueDef = "string" | "number" | "boolean";
type ValueDefs = Record<string, ValueDef>;

const valueDefMap: Record<
  ValueDef,
  StimulusValueDefs[keyof StimulusValueDefs]
> = {
  string: String,
  number: Number,
  boolean: Boolean,
};

type InferValueDef<T extends ValueDef> = T extends "string"
  ? string
  : T extends "number"
    ? number
    : T extends "boolean"
      ? boolean
      : never;

// eslint-disable-next-line @typescript-eslint/no-unused-vars -- Hush.
declare class ITypedController<
  TElement extends TargetDef,
  TTarget extends TargetDefs,
  TValue extends ValueDefs,
> extends Controller<InferTargetDef<TElement>> {
  declare static identifier: string;

  getTarget<K extends keyof TTarget>(name: K): InferTargetDef<TTarget[K]>;
  getValue<K extends keyof TValue>(name: K): InferValueDef<TValue[K]>;
}

export function TypedController<
  TElement extends TargetDef,
  TTarget extends TargetDefs,
  TValue extends ValueDefs,
>(
  identifier: string,
  _element: TElement,
  options: {
    targets?: TTarget;
    values?: TValue;
  },
): typeof ITypedController<TElement, TTarget, TValue> {
  const targets = Object.keys(options.targets ?? []);
  const valueDefs = options.values;
  const stimulusValueDefs: StimulusValueDefs = Object.fromEntries(
    Object.entries(valueDefs ?? {}).map(([key, valueDef]) => {
      return [key, valueDefMap[valueDef]];
    }),
  );

  return class TypedController extends Controller<InferTargetDef<TElement>> {
    static identifier = identifier;
    static targets = targets;
    static values = stimulusValueDefs;

    getTarget<K extends keyof TTarget>(name: K): InferTargetDef<TTarget[K]> {
      const key = `${String(name)}Target` as keyof Controller;
      // NOTE: Stimulus automatically throws if the target is not found.
      return this[key] as never;
    }

    getValue<K extends keyof TValue>(name: K): InferValueDef<TValue[K]> {
      const existentialKey =
        `has${capitalize(String(name))}Value` as keyof Controller;
      const exists = this[existentialKey] as unknown as boolean;
      if (!exists) throw new Error(`Missing value: ${String(name)}`);

      const getterKey = `${String(name)}Value` as keyof Controller;
      const value = this[getterKey] as never;
      return value;
    }
  };
}

function capitalize(key: string): string {
  return key.charAt(0).toUpperCase() + key.slice(1);
}
