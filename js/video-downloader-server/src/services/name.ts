export interface NameResolver {
  resolveName(url: string): Promise<string>;
}

export class DefaultNameResolver implements NameResolver {
  async resolveName(url: string): Promise<string> {
    throw new Error('Not implemented');
  }
}