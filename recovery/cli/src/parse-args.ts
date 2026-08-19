export function parseArgs(argv: string[]) {
  const flags: Record<string, string> = {}
  const positional: string[] = []

  for (let i = 0; i < argv.length; i++) {
    const token = argv[i]
    if (token === "--help") {
      flags.help = "true"
      continue
    }
    if (!token.startsWith("--")) {
      positional.push(token)
      continue
    }
    const key = token.slice(2)
    const next = argv[i + 1]
    if (!next || next.startsWith("--")) {
      flags[key] = "true"
      continue
    }
    flags[key] = next
    i++
  }

  return {
    command: positional[0],
    flags,
  }
}
