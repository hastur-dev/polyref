import { extractCallsFromSource } from './ast_extractor';
import { parseReferenceFile } from './ref_parser';
import {
  EntryKind,
  Severity,
  type CallSite,
  type Issue,
  type ReferenceEntry,
  type ReferenceFile,
  type ValidationResult,
} from './models';

/**
 * Check TypeScript source code against reference files.
 */
export function checkSource(
  source: string,
  referenceFiles: ReferenceFile[],
  filePath = '<stdin>'
): ValidationResult {
  const calls = extractCallsFromSource(source, filePath);
  const allMethods = collectAllMethods(referenceFiles);
  const allEntries = referenceFiles.flatMap(rf => rf.entries);
  const lines = source.split('\n');
  const issues: Issue[] = [];

  for (const call of calls) {
    if (call.callType !== 'method') continue;
    if (allMethods.length === 0) continue;

    const lineStr = lines[call.lineNumber - 1] ?? '';

    if (allMethods.includes(call.methodName)) {
      // Known method — check arg count
      const entry = allEntries.find(
        e =>
          e.name === call.methodName &&
          (e.kind === EntryKind.Method || e.kind === EntryKind.Function) &&
          (e.minArgs !== undefined || e.maxArgs !== undefined)
      );
      if (entry) {
        if (entry.minArgs !== undefined && call.argCount < entry.minArgs) {
          issues.push({
            severity: Severity.Error,
            message: `'${call.methodName}' expects at least ${entry.minArgs} arg(s), got ${call.argCount}`,
            file: filePath,
            line: call.lineNumber,
            codeSnippet: lineStr,
            suggestion: `${call.methodName} requires ${entry.minArgs} arg(s) minimum`,
            rule: 'too-few-args',
          });
        }
        if (entry.maxArgs !== undefined && call.argCount > entry.maxArgs) {
          issues.push({
            severity: Severity.Error,
            message: `'${call.methodName}' expects at most ${entry.maxArgs} arg(s), got ${call.argCount}`,
            file: filePath,
            line: call.lineNumber,
            codeSnippet: lineStr,
            suggestion: `${call.methodName} accepts ${entry.maxArgs} arg(s) maximum`,
            rule: 'too-many-args',
          });
        }
      }
    } else {
      // Unknown method — suggest closest match
      const suggestion = findBestSuggestion(call.methodName, allMethods);
      issues.push({
        severity: Severity.Error,
        message: `'${call.methodName}' is not a known method`,
        file: filePath,
        line: call.lineNumber,
        codeSnippet: lineStr,
        suggestion: suggestion
          ? `did you mean '${suggestion}'?`
          : 'no close match found — verify against docs',
        rule: 'unknown-method',
      });
    }
  }

  return {
    language: 'typescript',
    filesChecked: 1,
    issues,
  };
}

function collectAllMethods(refs: ReferenceFile[]): string[] {
  const methods = new Set<string>();
  for (const rf of refs) {
    for (const e of rf.entries) {
      if (e.kind === EntryKind.Method || e.kind === EntryKind.Function) {
        methods.add(e.name);
      }
    }
  }
  return Array.from(methods).sort();
}

function findBestSuggestion(name: string, candidates: string[]): string | null {
  let best = '';
  let bestScore = 0;

  for (const candidate of candidates) {
    const score = jaroWinkler(name, candidate);
    if (score > bestScore) {
      bestScore = score;
      best = candidate;
    }
  }

  return bestScore >= 0.35 ? best : null;
}

function jaroWinkler(s1: string, s2: string): number {
  if (s1 === s2) return 1.0;
  if (s1.length === 0 || s2.length === 0) return 0.0;

  const matchWindow = Math.max(Math.floor(Math.max(s1.length, s2.length) / 2) - 1, 0);
  const s1Matches = new Array(s1.length).fill(false);
  const s2Matches = new Array(s2.length).fill(false);

  let matches = 0;
  let transpositions = 0;

  for (let i = 0; i < s1.length; i++) {
    const start = Math.max(0, i - matchWindow);
    const end = Math.min(i + matchWindow + 1, s2.length);
    for (let j = start; j < end; j++) {
      if (s2Matches[j] || s1[i] !== s2[j]) continue;
      s1Matches[i] = true;
      s2Matches[j] = true;
      matches++;
      break;
    }
  }

  if (matches === 0) return 0.0;

  let k = 0;
  for (let i = 0; i < s1.length; i++) {
    if (!s1Matches[i]) continue;
    while (!s2Matches[k]) k++;
    if (s1[i] !== s2[k]) transpositions++;
    k++;
  }

  const jaro =
    (matches / s1.length + matches / s2.length + (matches - transpositions / 2) / matches) / 3;

  let prefix = 0;
  for (let i = 0; i < Math.min(4, Math.min(s1.length, s2.length)); i++) {
    if (s1[i] === s2[i]) prefix++;
    else break;
  }

  return jaro + prefix * 0.1 * (1 - jaro);
}
