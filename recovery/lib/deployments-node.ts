import { readFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import type { Deployments } from "./types"

const moduleDir = dirname(fileURLToPath(import.meta.url))

export function loadDeployments(path?: string): Deployments {
  const file = path ?? join(moduleDir, "../../docs/deployments.json")
  return JSON.parse(readFileSync(file, "utf8")) as Deployments
}
