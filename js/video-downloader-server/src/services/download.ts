import type { CompletedDownload } from '../schemas.js';

export interface DownloadJob {
  jobId: string;
  url: string;
  name: string;
  enqueuedAt: Date;
}

export class DownloadService {
  private queue: DownloadJob[] = [];
  private completed: CompletedDownload[] = [];
  private processing = false;

  enqueue(url: string, name: string): string {
    const jobId = this.generateJobId();
    const job: DownloadJob = {
      jobId,
      url,
      name,
      enqueuedAt: new Date(),
    };
    this.queue.push(job);
    this.processQueue();
    return jobId;
  }

  getCompleted(): CompletedDownload[] {
    return [...this.completed];
  }

  private generateJobId(): string {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    for (let i = 0; i < 8; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  private async processQueue(): Promise<void> {
    if (this.processing) return;
    this.processing = true;

    while (this.queue.length > 0) {
      const job = this.queue.shift()!;
      await this.processJob(job);
    }

    this.processing = false;
  }

  private async processJob(job: DownloadJob): Promise<void> {
    throw new Error('Not implemented');
  }
}