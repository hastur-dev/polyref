import * as fs from 'fs';
import * as path from 'path';
import { checkSource } from './checker';
import { parseReferenceFile } from './ref_parser';
import type { ReferenceFile, ValidationResult } from './models';

interface CliArgs {
  source?: string;
  fromStdin: boolean;
  refsDir: string;
  outputFormat: 'human' | 'json';
  enforce: boolean;
}

function parseArgs(argv: string[]): CliArgs {
  const args: CliArgs = {
    fromStdin: false,
    refsDir: 'refs',
    outputFormat: 'human',
    enforce: false,
  };

  for (let i = 2; i < argv.length; i++) {
    switch (argv[i]) {
      case '--from-stdin':
        args.fromStdin = true;
        break;
      case '--refs':
        args.refsDir = argv[++i];
        break;
      case '--output-format':
        args.outputFormat = argv[++i] as 'human' | 'json';
        break;
      case '--enforce':
        args.enforce = true;
        break;
      default:
        if (!argv[i].startsWith('--')) {
          args.source = argv[i];
        }
        break;
    }
  }

  return args;
}

function loadRefFiles(refsDir: string): ReferenceFile[] {
  const refs: ReferenceFile[] = [];
  const tsDir = path.join(refsDir, 'ts');

  if (fs.existsSync(tsDir)) {
    for (const file of fs.readdirSync(tsDir)) {
      const filePath = path.join(tsDir, file);
      if (!fs.statSync(filePath).isFile()) continue;
      const content = fs.readFileSync(filePath, 'utf-8');
      const ref = parseReferenceFile(content);
      refs.push({ ...ref, filePath });
    }
  }

  // Also scan refs/std/ for stdlib refs
  const stdDir = path.join(refsDir, 'std');
  if (fs.existsSync(stdDir)) {
    for (const file of fs.readdirSync(stdDir)) {
      if (!file.endsWith('.polyref')) continue;
      const filePath = path.join(stdDir, file);
      const content = fs.readFileSync(filePath, 'utf-8');
      const ref = parseReferenceFile(content);
      if (ref.language === 'typescript') {
        refs.push({ ...ref, filePath });
      }
    }
  }

  return refs;
}

function formatHuman(result: ValidationResult): string {
  if (result.issues.length === 0) {
    return 'APPROVED — no issues found';
  }

  const lines: string[] = [];
  for (const issue of result.issues) {
    lines.push(`${issue.severity.toUpperCase()}: ${issue.file}:${issue.line} — ${issue.message}`);
    if (issue.suggestion) {
      lines.push(`  suggestion: ${issue.suggestion}`);
    }
  }
  lines.push(`\n${result.issues.length} issue(s) found`);
  return lines.join('\n');
}

export function main(argv: string[] = process.argv): void {
  const args = parseArgs(argv);

  let source: string;
  let filePath: string;

  if (args.fromStdin) {
    source = fs.readFileSync(0, 'utf-8');
    filePath = '<stdin>';
  } else if (args.source) {
    source = fs.readFileSync(args.source, 'utf-8');
    filePath = args.source;
  } else {
    console.error('Error: no source file specified (use --from-stdin or provide a file path)');
    process.exit(1);
  }

  const refs = loadRefFiles(args.refsDir);
  const result = checkSource(source, refs, filePath);

  if (args.outputFormat === 'json') {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(formatHuman(result));
  }

  if (args.enforce && result.issues.length > 0) {
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}
