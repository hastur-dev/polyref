import { EntryKind, type ReferenceEntry, type ReferenceFile } from './models';

/**
 * Parse a .polyref v2 reference file.
 */
export function parseReferenceFile(content: string): ReferenceFile {
  const lines = content.split('\n');
  let libraryName = '';
  let version = '';
  let language = '';
  const entries: ReferenceEntry[] = [];
  let currentClass: string | null = null;

  for (const rawLine of lines) {
    const line = rawLine.trim();

    // Header comments
    const libMatch = line.match(/^#\s*Library:\s*(.+)/);
    if (libMatch) {
      libraryName = libMatch[1].trim();
      continue;
    }
    const verMatch = line.match(/^#\s*Version:\s*(.+)/);
    if (verMatch) {
      version = verMatch[1].trim();
      continue;
    }

    if (line.startsWith('@lang ')) {
      language = line.slice(6).trim();
      continue;
    }

    if (line.startsWith('@module ')) {
      entries.push({
        name: line.slice(8).trim(),
        kind: EntryKind.Module,
      });
      continue;
    }

    if (line.startsWith('@class ')) {
      const className = line.slice(7).replace('{', '').trim();
      currentClass = className;
      entries.push({
        name: className,
        kind: EntryKind.Class,
      });
      continue;
    }

    if (line === '}') {
      currentClass = null;
      continue;
    }

    if (line.startsWith('@method ')) {
      const entry = parseMethodOrFn(line.slice(8), EntryKind.Method, currentClass);
      if (entry) entries.push(entry);
      continue;
    }

    if (line.startsWith('@fn ')) {
      const entry = parseMethodOrFn(line.slice(4), EntryKind.Function, null);
      if (entry) entries.push(entry);
      continue;
    }

    if (line.startsWith('@field ')) {
      const fieldDef = line.slice(7).trim();
      const colonIdx = fieldDef.indexOf(':');
      const name = colonIdx > 0 ? fieldDef.slice(0, colonIdx).trim() : fieldDef;
      entries.push({
        name,
        kind: EntryKind.Field,
        typeContext: currentClass ?? undefined,
      });
      continue;
    }

    if (line.startsWith('@exception ')) {
      entries.push({
        name: line.slice(11).trim(),
        kind: EntryKind.Exception,
      });
      continue;
    }

    if (line.startsWith('@constant ')) {
      entries.push({
        name: line.slice(10).trim(),
        kind: EntryKind.Constant,
      });
      continue;
    }
  }

  return {
    libraryName,
    version,
    language,
    entries,
    rawContent: content,
    filePath: '',
  };
}

function parseMethodOrFn(
  rest: string,
  kind: EntryKind,
  typeContext: string | null
): ReferenceEntry | null {
  // Extract arg counts from [min_args=N, max_args=M]
  let minArgs: number | undefined;
  let maxArgs: number | undefined;
  const bracketMatch = rest.match(/\[([^\]]+)\]/);
  if (bracketMatch) {
    const attrs = bracketMatch[1];
    const minMatch = attrs.match(/min_args\s*=\s*(\d+)/);
    const maxMatch = attrs.match(/max_args\s*=\s*(\d+)/);
    if (minMatch) minArgs = parseInt(minMatch[1], 10);
    if (maxMatch) maxArgs = parseInt(maxMatch[1], 10);
  }

  // Extract function name (before the opening paren)
  const nameMatch = rest.match(/^(\w+)\s*\(/);
  if (!nameMatch) {
    // Try: name(self, ...) pattern
    const altMatch = rest.match(/^(\w+)/);
    if (!altMatch) return null;
    return {
      name: altMatch[1],
      kind,
      typeContext: typeContext ?? undefined,
      minArgs,
      maxArgs,
    };
  }

  const name = nameMatch[1];
  // Skip 'self' parameter name
  if (name === 'self') return null;

  return {
    name,
    kind,
    signature: rest.replace(/\[.*\]/, '').trim(),
    typeContext: typeContext ?? undefined,
    minArgs,
    maxArgs,
  };
}

/**
 * Parse a reference file from disk.
 */
export function parseReferenceFileFromPath(filePath: string): ReferenceFile {
  const fs = require('fs');
  const content = fs.readFileSync(filePath, 'utf-8');
  const ref = parseReferenceFile(content);
  return { ...ref, filePath };
}
