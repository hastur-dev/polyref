/** The kind of entry in a reference file. */
export enum EntryKind {
  Function = 'function',
  Method = 'method',
  Class = 'class',
  Module = 'module',
  Field = 'field',
  Exception = 'exception',
  AssociatedFn = 'associated_fn',
  EnumVariant = 'enum_variant',
  Constant = 'constant',
  Type = 'type',
}

/** Severity of a validation issue. */
export enum Severity {
  Info = 'info',
  Warning = 'warning',
  Error = 'error',
}

/** A single entry from a reference file. */
export interface ReferenceEntry {
  name: string;
  kind: EntryKind;
  signature?: string;
  description?: string;
  typeContext?: string;
  minArgs?: number;
  maxArgs?: number;
  modulePath?: string;
}

/** A parsed reference file for one library. */
export interface ReferenceFile {
  libraryName: string;
  version: string;
  language: string;
  entries: ReferenceEntry[];
  rawContent: string;
  filePath: string;
}

/** A single validation issue found in source code. */
export interface Issue {
  severity: Severity;
  message: string;
  file: string;
  line: number;
  column?: number;
  codeSnippet: string;
  suggestion?: string;
  rule: string;
}

/** Result of validating source files. */
export interface ValidationResult {
  language: string;
  filesChecked: number;
  issues: Issue[];
}

/** A call site extracted from source code via AST parsing. */
export interface CallSite {
  callType: 'method' | 'function' | 'constructor';
  receiver: string;
  methodName: string;
  lineNumber: number;
  argCount: number;
}

/** Create a ReferenceEntry with defaults. */
export function createEntry(
  name: string,
  kind: EntryKind,
  options?: Partial<Omit<ReferenceEntry, 'name' | 'kind'>>
): ReferenceEntry {
  return { name, kind, ...options };
}

/** Create a ReferenceFile with defaults. */
export function createReferenceFile(
  libraryName: string,
  entries: ReferenceEntry[],
  options?: Partial<Omit<ReferenceFile, 'libraryName' | 'entries'>>
): ReferenceFile {
  return {
    libraryName,
    entries,
    version: options?.version ?? 'unknown',
    language: options?.language ?? 'typescript',
    rawContent: options?.rawContent ?? '',
    filePath: options?.filePath ?? '',
  };
}

/** Check if an issue is an error. */
export function isError(issue: Issue): boolean {
  return issue.severity === Severity.Error;
}

/** Count errors in a validation result. */
export function errorCount(result: ValidationResult): number {
  return result.issues.filter(isError).length;
}

/** Check if validation result is clean (no errors). */
export function isClean(result: ValidationResult): boolean {
  return errorCount(result) === 0;
}
