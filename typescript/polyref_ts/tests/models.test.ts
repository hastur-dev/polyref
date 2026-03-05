import {
  EntryKind,
  Severity,
  createEntry,
  createReferenceFile,
  isError,
  errorCount,
  isClean,
  type ReferenceEntry,
  type ReferenceFile,
  type Issue,
  type ValidationResult,
  type CallSite,
} from '../src/models';

describe('EntryKind', () => {
  test('has expected values', () => {
    expect(EntryKind.Function).toBe('function');
    expect(EntryKind.Method).toBe('method');
    expect(EntryKind.Class).toBe('class');
    expect(EntryKind.Module).toBe('module');
    expect(EntryKind.Field).toBe('field');
  });
});

describe('Severity', () => {
  test('has expected values', () => {
    expect(Severity.Info).toBe('info');
    expect(Severity.Warning).toBe('warning');
    expect(Severity.Error).toBe('error');
  });
});

describe('createEntry', () => {
  test('creates entry with required fields', () => {
    const entry = createEntry('push', EntryKind.Method);
    expect(entry.name).toBe('push');
    expect(entry.kind).toBe(EntryKind.Method);
    expect(entry.signature).toBeUndefined();
  });

  test('creates entry with optional fields', () => {
    const entry = createEntry('insert', EntryKind.Method, {
      minArgs: 2,
      maxArgs: 2,
      typeContext: 'Map',
    });
    expect(entry.name).toBe('insert');
    expect(entry.minArgs).toBe(2);
    expect(entry.maxArgs).toBe(2);
    expect(entry.typeContext).toBe('Map');
  });
});

describe('createReferenceFile', () => {
  test('creates ref file with defaults', () => {
    const entries = [createEntry('test', EntryKind.Function)];
    const rf = createReferenceFile('mylib', entries);
    expect(rf.libraryName).toBe('mylib');
    expect(rf.entries).toHaveLength(1);
    expect(rf.version).toBe('unknown');
    expect(rf.language).toBe('typescript');
  });

  test('creates ref file with overrides', () => {
    const rf = createReferenceFile('mylib', [], {
      version: '2.0',
      language: 'javascript',
    });
    expect(rf.version).toBe('2.0');
    expect(rf.language).toBe('javascript');
  });
});

describe('Issue helpers', () => {
  const makeIssue = (severity: Severity): Issue => ({
    severity,
    message: 'test',
    file: 'test.ts',
    line: 1,
    codeSnippet: '',
    rule: 'test-rule',
  });

  test('isError identifies errors', () => {
    expect(isError(makeIssue(Severity.Error))).toBe(true);
    expect(isError(makeIssue(Severity.Warning))).toBe(false);
    expect(isError(makeIssue(Severity.Info))).toBe(false);
  });

  test('errorCount counts only errors', () => {
    const result: ValidationResult = {
      language: 'typescript',
      filesChecked: 1,
      issues: [
        makeIssue(Severity.Error),
        makeIssue(Severity.Warning),
        makeIssue(Severity.Error),
        makeIssue(Severity.Info),
      ],
    };
    expect(errorCount(result)).toBe(2);
  });

  test('isClean returns true when no errors', () => {
    const clean: ValidationResult = {
      language: 'typescript',
      filesChecked: 1,
      issues: [makeIssue(Severity.Warning)],
    };
    expect(isClean(clean)).toBe(true);
  });

  test('isClean returns false when errors exist', () => {
    const dirty: ValidationResult = {
      language: 'typescript',
      filesChecked: 1,
      issues: [makeIssue(Severity.Error)],
    };
    expect(isClean(dirty)).toBe(false);
  });
});

describe('CallSite type', () => {
  test('can create method call site', () => {
    const site: CallSite = {
      callType: 'method',
      receiver: 'arr',
      methodName: 'push',
      lineNumber: 5,
      argCount: 1,
    };
    expect(site.callType).toBe('method');
    expect(site.receiver).toBe('arr');
    expect(site.methodName).toBe('push');
    expect(site.lineNumber).toBe(5);
    expect(site.argCount).toBe(1);
  });
});
