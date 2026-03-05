import { parseReferenceFile } from '../src/ref_parser';
import { EntryKind } from '../src/models';

const SAMPLE_POLYREF = `
# ============================================================
# Library: express
# Version: 4.18
# Lang: typescript
# ============================================================
@lang typescript

@module express

@class Application {
    @method listen(port: number, callback?: Function) -> Server  [min_args=1, max_args=2]
    @method get(path: string, ...handlers: Function[]) -> Application  [min_args=1]
    @method post(path: string, ...handlers: Function[]) -> Application  [min_args=1]
    @method use(...handlers: Function[]) -> Application  [min_args=0]
    @field locals: Record<string, any>
}

@class Response {
    @method json(body: any) -> Response  [min_args=1, max_args=1]
    @method send(body?: any) -> Response  [min_args=0, max_args=1]
    @method status(code: number) -> Response  [min_args=1, max_args=1]
    @method redirect(url: string) -> void  [min_args=1, max_args=1]
}

@fn express() -> Application  [min_args=0, max_args=0]
`;

describe('parseReferenceFile', () => {
  test('parses library metadata', () => {
    const ref = parseReferenceFile(SAMPLE_POLYREF);
    expect(ref.libraryName).toBe('express');
    expect(ref.version).toBe('4.18');
    expect(ref.language).toBe('typescript');
  });

  test('parses modules', () => {
    const ref = parseReferenceFile(SAMPLE_POLYREF);
    const modules = ref.entries.filter(e => e.kind === EntryKind.Module);
    expect(modules).toHaveLength(1);
    expect(modules[0].name).toBe('express');
  });

  test('parses classes', () => {
    const ref = parseReferenceFile(SAMPLE_POLYREF);
    const classes = ref.entries.filter(e => e.kind === EntryKind.Class);
    expect(classes.map(c => c.name)).toContain('Application');
    expect(classes.map(c => c.name)).toContain('Response');
  });

  test('parses methods with arg counts', () => {
    const ref = parseReferenceFile(SAMPLE_POLYREF);
    const listen = ref.entries.find(e => e.name === 'listen' && e.kind === EntryKind.Method);
    expect(listen).toBeDefined();
    expect(listen!.minArgs).toBe(1);
    expect(listen!.maxArgs).toBe(2);
    expect(listen!.typeContext).toBe('Application');
  });

  test('parses functions', () => {
    const ref = parseReferenceFile(SAMPLE_POLYREF);
    const express = ref.entries.find(e => e.name === 'express' && e.kind === EntryKind.Function);
    expect(express).toBeDefined();
    expect(express!.minArgs).toBe(0);
    expect(express!.maxArgs).toBe(0);
  });

  test('parses fields', () => {
    const ref = parseReferenceFile(SAMPLE_POLYREF);
    const locals = ref.entries.find(e => e.name === 'locals' && e.kind === EntryKind.Field);
    expect(locals).toBeDefined();
    expect(locals!.typeContext).toBe('Application');
  });
});
