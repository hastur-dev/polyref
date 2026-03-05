import * as ts from 'typescript';
import type { CallSite } from './models';

/**
 * Extract all call sites from TypeScript source code using the TypeScript compiler API.
 */
export function extractCallsFromSource(source: string, fileName = 'input.ts'): CallSite[] {
  const sourceFile = ts.createSourceFile(fileName, source, ts.ScriptTarget.Latest, true);
  const calls: CallSite[] = [];

  function visit(node: ts.Node): void {
    if (ts.isCallExpression(node)) {
      const site = extractCallSite(node, sourceFile);
      if (site) {
        calls.push(site);
      }
    }
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return calls;
}

function extractCallSite(node: ts.CallExpression, sourceFile: ts.SourceFile): CallSite | null {
  const argCount = node.arguments.length;
  const lineNumber = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;

  if (ts.isPropertyAccessExpression(node.expression)) {
    // receiver.method() or receiver.property.method()
    const methodName = node.expression.name.text;
    const receiver = extractReceiver(node.expression.expression, sourceFile);
    return {
      callType: 'method',
      receiver,
      methodName,
      lineNumber,
      argCount,
    };
  }

  if (ts.isIdentifier(node.expression)) {
    // freeFunction()
    return {
      callType: 'function',
      receiver: '',
      methodName: node.expression.text,
      lineNumber,
      argCount,
    };
  }

  if (ts.isNewExpression(node.parent) === false && ts.isIdentifier(node.expression)) {
    return null;
  }

  return null;
}

function extractReceiver(expr: ts.Expression, sourceFile: ts.SourceFile): string {
  if (ts.isIdentifier(expr)) {
    return expr.text;
  }
  if (ts.isPropertyAccessExpression(expr)) {
    const base = extractReceiver(expr.expression, sourceFile);
    return base ? `${base}.${expr.name.text}` : expr.name.text;
  }
  if (expr.kind === ts.SyntaxKind.ThisKeyword) {
    return 'this';
  }
  return '';
}

/**
 * Extract call sites from a TypeScript/JavaScript file on disk.
 */
export function extractCallsFromFile(filePath: string): CallSite[] {
  const fs = require('fs');
  const source = fs.readFileSync(filePath, 'utf-8');
  return extractCallsFromSource(source, filePath);
}
