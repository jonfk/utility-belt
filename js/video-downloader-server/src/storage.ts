import { promises as fs } from 'fs';
import path from 'path';
import { config } from './config.js';

export async function ensureDataDir(): Promise<void> {
  try {
    await fs.access(config.dataDir);
  } catch {
    await fs.mkdir(config.dataDir, { recursive: true });
  }
}

export function sanitizeFilename(filename: string): string {
  return filename.replace(/[^a-zA-Z0-9._-]/g, '_');
}

export function getSafeFilePath(filename: string): string {
  const sanitized = sanitizeFilename(filename);
  return path.join(config.dataDir, sanitized);
}

export async function writeFile(filePath: string, data: Buffer): Promise<void> {
  await fs.writeFile(filePath, data);
}

export async function getFileSize(filePath: string): Promise<number> {
  const stats = await fs.stat(filePath);
  return stats.size;
}