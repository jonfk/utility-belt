#!/usr/bin/env bun
import { cac } from 'cac'
import { renderMermaid, renderMermaidAscii, THEMES } from 'beautiful-mermaid'
import path from 'node:path'

const VERSION = '0.1.0'

type RenderTheme = {
  bg?: string
  fg?: string
  line?: string
  accent?: string
  muted?: string
  surface?: string
  border?: string
}

function fail(message: string, code = 1): never {
  console.error(`Error: ${message}`)
  process.exit(code)
}

function listThemes(): void {
  const names = Object.keys(THEMES).sort()
  if (names.length === 0) {
    console.log('No themes are available in this version of beautiful-mermaid.')
    return
  }

  console.log(names.join('\n'))
}

function deriveSvgPath(inputPath: string): string {
  const parsed = path.parse(inputPath)
  const base = parsed.ext ? parsed.name : parsed.base
  const dir = parsed.dir || '.'
  return path.join(dir, `${base}.svg`)
}

async function readInput(inputPath?: string): Promise<{ content: string; sourcePath?: string }> {
  if (inputPath && inputPath !== '-') {
    const file = Bun.file(inputPath)
    if (!await file.exists()) {
      fail(`Input file not found: ${inputPath}`, 2)
    }
    return { content: await file.text(), sourcePath: inputPath }
  }

  if (process.stdin.isTTY) {
    fail('No input provided. Pass a file path or pipe Mermaid text to stdin.')
  }

  return { content: await Bun.stdin.text() }
}

function normalizeMermaidInput(content: string): string {
  const lines = content.split(/\r?\n/)
  const firstIndex = lines.findIndex((line) => line.trim().length > 0)
  if (firstIndex === -1) {
    return content
  }

  const line = lines[firstIndex] ?? ''
  const match = line.match(/^\s*(graph|flowchart)\s+(TD|TB|LR|BT|RL)\s*;(.*)$/i)
  if (!match) {
    return content
  }

  const semicolonIndex = line.indexOf(';')
  if (semicolonIndex === -1) {
    return content
  }

  const header = line.slice(0, semicolonIndex).trimEnd()
  const rest = line.slice(semicolonIndex + 1).trim()
  lines[firstIndex] = header
  if (rest.length > 0) {
    lines.splice(firstIndex + 1, 0, rest)
  }

  return lines.join('\n')
}

async function writeToStdout(content: string): Promise<void> {
  await Bun.write(Bun.stdout, content)
  if (!content.endsWith('\n')) {
    await Bun.write(Bun.stdout, '\n')
  }
}

async function writeToFile(outputPath: string, content: string): Promise<void> {
  await Bun.write(outputPath, content)
}

const cli = cac('beautiful-mermaid')

cli
  .command('[...files]', 'Render Mermaid diagrams as SVG (default) or ASCII')
  .option('-a, --ascii', 'Output ASCII instead of SVG')
  .option('--svg', 'Force SVG output (default)')
  .option('-o, --output <file>', 'Write output to a file')
  .option('-t, --theme <name>', 'Apply a built-in theme for SVG output')
  .option('--themes', 'List available themes and exit')
  .action(async (files: string[] = [], options) => {
    if (options.themes) {
      listThemes()
      return
    }

    if (files.length > 1) {
      fail('Only one input file is supported.')
    }

    if (options.ascii && options.svg) {
      fail('Choose either --ascii or --svg, not both.')
    }

    const format = options.ascii ? 'ascii' : 'svg'

    if (format === 'ascii' && options.theme) {
      fail('--theme is only valid for SVG output.')
    }

    const inputPath = files[0]
    const { content: rawContent, sourcePath } = await readInput(inputPath)
    const content = normalizeMermaidInput(rawContent)

    if (!content.trim()) {
      fail('Input is empty.')
    }

    if (format === 'svg') {
      let outputPath: string | undefined = options.output
      if (!outputPath) {
        if (!sourcePath) {
          fail('SVG output from stdin requires --output <file>.')
        }
        outputPath = deriveSvgPath(sourcePath)
      }

      let theme: RenderTheme | undefined
      if (options.theme) {
        const resolved = THEMES[options.theme as keyof typeof THEMES]
        if (!resolved) {
          fail(`Unknown theme "${options.theme}". Use --themes to list options.`)
        }
        theme = {
          bg: resolved.bg,
          fg: resolved.fg,
          line: resolved.line,
          accent: resolved.accent,
          muted: resolved.muted,
          surface: resolved.surface,
          border: resolved.border,
        }
      }

      const svg = await renderMermaid(content, theme)
      await writeToFile(outputPath, svg)
      return
    }

    const ascii = await Promise.resolve(renderMermaidAscii(content))
    if (options.output) {
      await writeToFile(options.output, ascii)
      return
    }

    await writeToStdout(ascii)
  })

cli.help()
cli.version(VERSION)
cli.parse()
