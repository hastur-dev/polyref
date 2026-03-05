import { checkSource } from '../src/checker';
import { parseReferenceFile } from '../src/ref_parser';
import { EntryKind, type ReferenceFile, createEntry, createReferenceFile } from '../src/models';

function makeRef(): ReferenceFile {
  return createReferenceFile('testlib', [
    createEntry('push', EntryKind.Method, { minArgs: 1, maxArgs: 1 }),
    createEntry('pop', EntryKind.Method, { minArgs: 0, maxArgs: 0 }),
    createEntry('insert', EntryKind.Method, { minArgs: 2, maxArgs: 2 }),
    createEntry('clear', EntryKind.Method, { minArgs: 0, maxArgs: 0 }),
  ]);
}

describe('checkSource', () => {
  test('detects unknown method', () => {
    const source = `const x = arr.nonexistent();`;
    const result = checkSource(source, [makeRef()]);
    const unknownIssues = result.issues.filter(i => i.rule === 'unknown-method');
    expect(unknownIssues.length).toBeGreaterThan(0);
    expect(unknownIssues[0].message).toContain('nonexistent');
  });

  test('accepts known method', () => {
    const source = `arr.push(1);`;
    const result = checkSource(source, [makeRef()]);
    const pushIssues = result.issues.filter(i => i.message.includes('push'));
    expect(pushIssues).toHaveLength(0);
  });

  test('detects too few args', () => {
    const source = `arr.insert();`;
    const result = checkSource(source, [makeRef()]);
    const argIssues = result.issues.filter(i => i.rule === 'too-few-args');
    expect(argIssues.length).toBeGreaterThan(0);
  });

  test('detects too many args', () => {
    const source = `arr.clear(1, 2, 3);`;
    const result = checkSource(source, [makeRef()]);
    const argIssues = result.issues.filter(i => i.rule === 'too-many-args');
    expect(argIssues.length).toBeGreaterThan(0);
  });

  test('no issues for correct calls', () => {
    const source = `arr.push(1); arr.pop(); arr.insert("a", "b");`;
    const result = checkSource(source, [makeRef()]);
    expect(result.issues).toHaveLength(0);
  });

  test('works with no refs', () => {
    const source = `arr.whatever();`;
    const result = checkSource(source, []);
    expect(result.issues).toHaveLength(0);
  });

  test('parses and checks against polyref content', () => {
    const content = `
@lang typescript
@module test
@class Foo {
    @method bar(x: number) -> void  [min_args=1, max_args=1]
}
`;
    const ref = parseReferenceFile(content);
    const source = `foo.bar();`;
    const result = checkSource(source, [ref]);
    const tooFew = result.issues.filter(i => i.rule === 'too-few-args');
    expect(tooFew.length).toBeGreaterThan(0);
  });

  test('provides suggestion for unknown method', () => {
    const source = `arr.pus();`;
    const result = checkSource(source, [makeRef()]);
    const issue = result.issues.find(i => i.rule === 'unknown-method');
    expect(issue).toBeDefined();
    expect(issue!.suggestion).toContain('push');
  });
});
