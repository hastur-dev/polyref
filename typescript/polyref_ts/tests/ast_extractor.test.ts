import { extractCallsFromSource } from '../src/ast_extractor';

describe('extractCallsFromSource', () => {
  test('extracts method calls', () => {
    const source = `
const arr = [1, 2, 3];
arr.push(4);
arr.pop();
`;
    const calls = extractCallsFromSource(source);
    const push = calls.find(c => c.methodName === 'push');
    expect(push).toBeDefined();
    expect(push!.callType).toBe('method');
    expect(push!.receiver).toBe('arr');
    expect(push!.argCount).toBe(1);

    const pop = calls.find(c => c.methodName === 'pop');
    expect(pop).toBeDefined();
    expect(pop!.argCount).toBe(0);
  });

  test('extracts free function calls', () => {
    const source = `
console.log("hello");
parseInt("42", 10);
`;
    const calls = extractCallsFromSource(source);
    const parseInt_ = calls.find(c => c.methodName === 'parseInt');
    expect(parseInt_).toBeDefined();
    expect(parseInt_!.callType).toBe('function');
    expect(parseInt_!.argCount).toBe(2);
  });

  test('extracts chained method calls', () => {
    const source = `
const result = items.filter(x => x > 0).map(x => x * 2).join(', ');
`;
    const calls = extractCallsFromSource(source);
    const names = calls.map(c => c.methodName);
    expect(names).toContain('filter');
    expect(names).toContain('map');
    expect(names).toContain('join');
  });

  test('handles nested receiver', () => {
    const source = `
this.items.push(1);
`;
    const calls = extractCallsFromSource(source);
    const push = calls.find(c => c.methodName === 'push');
    expect(push).toBeDefined();
    expect(push!.receiver).toBe('this.items');
  });

  test('returns empty for no calls', () => {
    const source = `const x = 1 + 2;`;
    const calls = extractCallsFromSource(source);
    expect(calls).toHaveLength(0);
  });

  test('counts arguments correctly', () => {
    const source = `
map.set("key", "value");
`;
    const calls = extractCallsFromSource(source);
    const set = calls.find(c => c.methodName === 'set');
    expect(set).toBeDefined();
    expect(set!.argCount).toBe(2);
  });
});
