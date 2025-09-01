import type { CompletedDownload, DownloadProgress } from '../schemas.js';
import fs from 'fs';
import path from 'path';
import { pipeline } from 'stream/promises';
import { Readable, Transform } from 'stream';
import type { ReadableStream as NodeReadableStream } from 'stream/web';
import { EventEmitter } from 'events';
import type { FastifyBaseLogger } from 'fastify';
import { 
  UnsupportedUrlError, 
  VideoSourceNotFoundError, 
  NetworkError,
  DownloadFailedError 
} from '../errors.js';
import { getBrowser } from '../puppeteer.js';

export interface DownloadJob {
  jobId: string;
  url: string;
  name: string;
  enqueuedAt: Date;
}

interface Downloader {
  download(jobId: string, url: string, name: string, progressCallback?: (progress: DownloadProgress) => void): Promise<CompletedDownload>;
}

class SxyPrnDownloader implements Downloader {
  private logger: FastifyBaseLogger;

  constructor(logger: FastifyBaseLogger) {
    this.logger = logger;
  }

  async download(jobId: string, url: string, name: string, progressCallback?: (progress: DownloadProgress) => void): Promise<CompletedDownload> {
    const jobLogger = this.logger.child({ jobId, url, name });
    jobLogger.info('Starting video download from SxyPrn');
    
    const browser = await getBrowser();
    const page = await browser.newPage();
    
    try {
      jobLogger.debug('Navigating to video page');
      await page.goto(url, { waitUntil: 'networkidle0' });
      
      jobLogger.debug('Waiting for video player element');
      await page.waitForSelector('#player_el');
      
      jobLogger.debug('Extracting video source URL');
      const videoSrc = await page.$eval('#player_el', el => (el as HTMLVideoElement).src);
      if (!videoSrc) {
        throw new VideoSourceNotFoundError('Could not find video source on page');
      }
      
      jobLogger.info({ videoSrc }, 'Found video source, resolving canonical URL');
      const canonicalUrl = await this.getCanonicalUrl(videoSrc);
      const outputPath = path.join(process.cwd(), 'data', `${name}.mp4`);
      
      jobLogger.debug({ outputPath }, 'Creating output directory');
      await fs.promises.mkdir(path.dirname(outputPath), { recursive: true });
      
      const startedAt = new Date();
      jobLogger.info({ canonicalUrl, outputPath }, 'Starting video file download');
      const size = await this.downloadVideo(jobId, url, name, canonicalUrl, outputPath, progressCallback, jobLogger);
      const finishedAt = new Date();
      
      const downloadDuration = finishedAt.getTime() - startedAt.getTime();
      jobLogger.info({ 
        outputPath,
        size, 
        downloadDuration 
      }, 'Video download completed successfully');
      
      return {
        url,
        name,
        savedPath: outputPath,
        size,
        startedAt: startedAt.toISOString(),
        finishedAt: finishedAt.toISOString()
      };
    } catch (error) {
      jobLogger.error({ error }, 'Video download failed');
      if (error instanceof VideoSourceNotFoundError) {
        throw error;
      }
      throw new DownloadFailedError(`Failed to download video from ${url}`, { 
        originalError: error instanceof Error ? error.message : String(error) 
      });
    } finally {
      await page.close();
    }
  }
  
  private async getCanonicalUrl(videoSrc: string): Promise<string> {
    try {
      const response = await fetch(videoSrc, { method: 'HEAD' });
      return response.url || videoSrc;
    } catch (error) {
      throw new NetworkError('Could not resolve canonical URL for video source', {
        videoSrc,
        originalError: error instanceof Error ? error.message : String(error)
      });
    }
  }
  
  private async downloadVideo(
    jobId: string, 
    jobUrl: string, 
    name: string, 
    videoUrl: string, 
    outputPath: string, 
    progressCallback?: (progress: DownloadProgress) => void,
    jobLogger?: FastifyBaseLogger
  ): Promise<number> {
    try {
      const response = await fetch(videoUrl);
      if (!response.ok) {
        throw new NetworkError(`Failed to fetch video: ${response.status} ${response.statusText}`, {
          url: videoUrl,
          status: response.status,
          statusText: response.statusText
        });
      }
      
      if (!response.body) {
        throw new NetworkError('Response body is empty', { url: videoUrl });
      }

      const contentLength = response.headers.get('content-length');
      const totalBytes = contentLength ? parseInt(contentLength, 10) : undefined;
      
      let downloadedBytes = 0;
      let lastProgressPercent = 0;
      const startedAt = new Date();

      // Create progress tracking transform stream
      const progressTracker = new Transform({
        transform(chunk, encoding, callback) {
          downloadedBytes += chunk.length;
          
          if (progressCallback && totalBytes) {
            const progressPercent = Math.floor((downloadedBytes / totalBytes) * 100);
            
            // Only emit progress every 5% and at completion
            if (progressPercent >= lastProgressPercent + 5 || progressPercent === 100) {
              const progress: DownloadProgress = {
                jobId,
                url: jobUrl,
                name,
                status: 'downloading',
                progressPercent,
                downloadedBytes,
                totalBytes,
                lastUpdated: new Date().toISOString(),
                startedAt: startedAt.toISOString()
              };
              
              progressCallback(progress);
              
              if (jobLogger && progressPercent >= lastProgressPercent + 5) {
                jobLogger.info({
                  jobId,
                  progressPercent,
                  downloadedBytes,
                  totalBytes
                }, `Download progress: ${progressPercent}%`);
              }
              
              lastProgressPercent = progressPercent;
            }
          }
          
          callback(null, chunk);
        }
      });
      
      const writeStream = fs.createWriteStream(outputPath);
      
      await pipeline(
        Readable.fromWeb(response.body as NodeReadableStream),
        progressTracker,
        writeStream
      );
      
      const stats = await fs.promises.stat(outputPath);
      return stats.size;
    } catch (error) {
      if (error instanceof NetworkError) {
        throw error;
      }
      throw new DownloadFailedError('Failed to download and save video file', {
        url: videoUrl,
        outputPath,
        originalError: error instanceof Error ? error.message : String(error)
      });
    }
  }
}

export class DownloadService extends EventEmitter {
  private queue: DownloadJob[] = [];
  private completed: CompletedDownload[] = [];
  private inProgress: Map<string, DownloadProgress> = new Map();
  private sxyPrnDownloader: SxyPrnDownloader;
  private logger: FastifyBaseLogger;

  constructor(logger: FastifyBaseLogger) {
    super();
    this.logger = logger;
    this.sxyPrnDownloader = new SxyPrnDownloader(logger);
    this.startProcessor();
  }

  enqueue(url: string, name: string): string {
    const jobId = this.generateJobId();
    const job: DownloadJob = {
      jobId,
      url,
      name,
      enqueuedAt: new Date(),
    };
    this.queue.push(job);
    
    this.logger.info({
      jobId,
      url,
      name,
      queueLength: this.queue.length
    }, 'Download job enqueued');
    
    this.emit('jobAdded');
    return jobId;
  }

  getCompleted(): CompletedDownload[] {
    return [...this.completed];
  }

  getProgress(jobId: string): DownloadProgress | undefined {
    return this.inProgress.get(jobId);
  }

  getAllProgress(): DownloadProgress[] {
    return Array.from(this.inProgress.values());
  }

  private generateJobId(): string {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    for (let i = 0; i < 8; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  private async startProcessor(): Promise<void> {
    this.processLoop().catch(error => {
      this.logger.error({ error }, 'Download processor crashed');
    });
  }

  private async processLoop(): Promise<void> {
    while (true) {
      if (this.queue.length === 0) {
        await new Promise<void>(resolve => this.once('jobAdded', resolve));
        continue;
      }

      const job = this.queue.shift()!;
      await this.processJob(job);
    }
  }

  private async processJob(job: DownloadJob): Promise<void> {
    const startTime = Date.now();
    
    // Initialize progress tracking
    const initialProgress: DownloadProgress = {
      jobId: job.jobId,
      url: job.url,
      name: job.name,
      status: 'processing',
      progressPercent: 0,
      downloadedBytes: 0,
      totalBytes: undefined,
      lastUpdated: new Date().toISOString(),
      startedAt: new Date().toISOString()
    };
    
    this.inProgress.set(job.jobId, initialProgress);
    
    this.logger.info({
      jobId: job.jobId,
      url: job.url,
      name: job.name,
      enqueuedAt: job.enqueuedAt,
      waitTime: startTime - job.enqueuedAt.getTime()
    }, 'Starting download job processing');
    
    // Progress callback to update the progress map
    const progressCallback = (progress: DownloadProgress) => {
      this.inProgress.set(job.jobId, progress);
    };
    
    try {
      const urlObj = new URL(job.url);
      let downloader: Downloader;
      
      if (urlObj.hostname.includes('sxyprn.com')) {
        downloader = this.sxyPrnDownloader;
      } else {
        throw new UnsupportedUrlError(`URL hostname not supported: ${urlObj.hostname}`);
      }
      
      const result = await downloader.download(job.jobId, job.url, job.name, progressCallback);
      this.completed.push(result);
      
      // Remove from in-progress tracking
      this.inProgress.delete(job.jobId);
      
      const endTime = Date.now();
      const processingTime = endTime - startTime;
      
      this.logger.info({
        jobId: job.jobId,
        url: job.url,
        name: job.name,
        savedPath: result.savedPath,
        size: result.size,
        processingTime,
        startedAt: result.startedAt,
        finishedAt: result.finishedAt
      }, 'Download job completed successfully');
      
    } catch (error) {
      // Remove from in-progress tracking on failure
      this.inProgress.delete(job.jobId);
      
      const endTime = Date.now();
      const processingTime = endTime - startTime;
      
      this.logger.error({ 
        jobId: job.jobId, 
        url: job.url, 
        name: job.name,
        processingTime,
        error 
      }, 'Failed to process download job');
    }
  }
}