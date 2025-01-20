const puppeteer = require('puppeteer-core');
const chrome = require('chrome-launcher');
const sanitize = require('sanitize-filename');
const moment = require('moment');
const axios = require('axios');
const fs = require('fs');
const path = require('path');
const csv = require('csv-parse/sync');

async function getBrowser() {
    const chromePath = await chrome.getChromePath();
    return puppeteer.launch({
        executablePath: chromePath,
        headless: "new"
    });
}

async function processTitle(title, prefix = '') {
    // Log original title
    console.log('Original title:', title);
    
    // Add prefix if provided
    if (prefix) {
        title = `${prefix} ${title}`;
    }
    
    // Define promotional content patterns
    const promotionalPatterns = [
        // Square bracket enclosed promotional messages containing "backup", "visit", "watch"
        /\[[^\]]*(?:backup|visit|watch)[^\]]*\]/i,
        
        // Backup/Watch/Download variations with quality indicators
        /(?:backup|watch|download)(?:\s*\/\s*(?:watch|download))?\s*(?:hd|fhd|full\s*hd)(?:\s*:)?/i,
        
        // "Backup HD on Link" variations
        /backup\s+(?:hd|fhd|full\s*hd)\s+on\s+link/i,
        
        // Visit blog/website messages
        /(?:also\s+)?visit\s+(?:my\s+)?(?:blog|website)\s*[-:]?\s*(?:https?:\/\/[^\s]+)?(?:\s+for\s+backup[^a-z]+)/i,
        
        // Watch Online quality variations
        /watch\s+online\s+(?:hd|fhd|full\s*hd)(?:\s*:)?/i
    ];
    
    // Define patterns for quality indicators and redundant tags
    const cleanupPatterns = [
        // Quality and resolution indicators
        /\b(?:720|720p|1080|1080p|hd(?:porn)?)\b/gi,
        
        // Common tags and low-information strings
        /(?:^|\s)#?(?:ghost|internallink|link|dailyvids|0dayporn)\b/gi
    ];
    
    // Remove promotional patterns
    promotionalPatterns.forEach(pattern => {
        title = title.replace(pattern, '');
    });
    
    // Remove quality indicators and redundant tags
    cleanupPatterns.forEach(pattern => {
        title = title.replace(pattern, '');
    });
    
    // Remove URLs
    title = title.replace(/(?:https?|ftp):\/\/[\n\S]+/g, '');
    
    // Clean up multiple spaces
    title = title.replace(/\s+/g, ' ');
    
    // Trim whitespace
    title = title.trim();
    
    // Extract date if present (supports various formats)
    let dateMatch = title.match(/\b\d{4}[-./]\d{1,2}[-./]\d{1,2}\b|\b\d{1,2}[-./]\d{1,2}[-./]\d{4}\b/);
    let extractedDate = '';
    
    if (dateMatch) {
        const parsedDate = moment(dateMatch[0], ['YYYY-MM-DD', 'DD-MM-YYYY', 'MM-DD-YYYY', 'DD.MM.YYYY'], true);
        if (parsedDate.isValid()) {
            extractedDate = parsedDate.format('YYYY-MM-DD');
            // Remove the date from the title
            title = title.replace(dateMatch[0], '').trim();
        }
    }
    
    // Truncate to 200 characters
    title = title.substring(0, 200);
    
    // Add date to the beginning if found
    if (extractedDate) {
        title = `${extractedDate}_${title}`;
    }
    
    return sanitize(title);
}

async function downloadVideo(url, outputPath) {
    const response = await axios({
        url,
        method: 'GET',
        responseType: 'stream',
    });

    const totalLength = response.headers['content-length'];
    const writer = fs.createWriteStream(outputPath);
    
    let downloaded = 0;
    let lastLogTime = Date.now();
    let lastDownloaded = 0;

    response.data.on('data', (chunk) => {
        downloaded += chunk.length;
        
        // Update progress every 500ms
        const now = Date.now();
        if (now - lastLogTime > 500) {
            const timeDiff = (now - lastLogTime) / 1000; // Convert to seconds
            const bytesPerSec = (downloaded - lastDownloaded) / timeDiff;
            const percent = (downloaded / totalLength) * 100;
            
            // Calculate remaining bytes and time
            const remainingBytes = totalLength - downloaded;
            const estimatedSeconds = remainingBytes / bytesPerSec;
            
            // Format estimated time remaining
            let timeRemaining;
            if (estimatedSeconds > 3600) {
                timeRemaining = `${Math.round(estimatedSeconds / 3600)}h ${Math.round((estimatedSeconds % 3600) / 60)}m`;
            } else if (estimatedSeconds > 60) {
                timeRemaining = `${Math.round(estimatedSeconds / 60)}m ${Math.round(estimatedSeconds % 60)}s`;
            } else {
                timeRemaining = `${Math.round(estimatedSeconds)}s`;
            }
            
            // Calculate human-readable speed
            let speed;
            if (bytesPerSec > 1024 * 1024) {
                speed = `${(bytesPerSec / (1024 * 1024)).toFixed(2)} MB/s`;
            } else if (bytesPerSec > 1024) {
                speed = `${(bytesPerSec / 1024).toFixed(2)} KB/s`;
            } else {
                speed = `${bytesPerSec.toFixed(2)} B/s`;
            }
            
            // Clear line and update progress with time remaining
            process.stdout.write(`\rProgress: ${percent.toFixed(1)}% | Speed: ${speed} | ETA: ${timeRemaining}`);
            
            lastLogTime = now;
            lastDownloaded = downloaded;
        }
    });

    response.data.pipe(writer);

    return new Promise((resolve, reject) => {
        writer.on('finish', () => {
            process.stdout.write('\nDownload complete!\n');
            resolve();
        });
        writer.on('error', reject);
    });
}

async function getCanonicalUrl(videoSrc) {
    try {
        const response = await axios.head(videoSrc);
        return response.request.res.responseUrl || videoSrc;
    } catch (error) {
        console.warn('Could not get canonical URL, using original source:', error.message);
        return videoSrc;
    }
}

function getArgs() {
    const isPackaged = !process.argv[0].endsWith('node') && !process.argv[0].endsWith('node.exe');
    const args = isPackaged ? process.argv.slice(1) : process.argv.slice(2);
    
    // Parse for --file parameter
    const fileIndex = args.findIndex(arg => arg === '--file');
    if (fileIndex !== -1 && fileIndex + 1 < args.length) {
        return {
            isPackaged,
            mode: 'file',
            filePath: args[fileIndex + 1]
        };
    }
    
    // Regular URL mode
    return {
        isPackaged,
        mode: 'url',
        url: args[0],
        prefix: args[1] || ''
    };
}

function showHelp(isPackaged) {
    const command = isPackaged ? 'video-downloader' : 'node script.js';
    console.log(`
Usage: 
    ${command} [URL] [prefix]
    ${command} --file [filepath]

Arguments:
    URL        The video page URL to download from
    prefix     Optional text to prepend to the filename
    filepath   Path to a CSV file containing URLs and optional prefixes

Options:
    -h, --help  Show this help message
    --file      Process multiple URLs from a CSV file

CSV File Format:
    The CSV file should have a header row with "url,prefix"
    The prefix column is optional

Examples:
    ${command} https://example.com/video "My Video"
    ${command} --file videos.csv
`);
    process.exit(0);
}

async function processUrlList(filePath) {
    try {
        const fileContent = fs.readFileSync(filePath, 'utf-8');
        const records = csv.parse(fileContent, {
            columns: true,
            skip_empty_lines: true,
            trim: true
        });
        
        return records.map(record => ({
            url: record.url,
            prefix: record.prefix || ''
        }));
    } catch (error) {
        throw new Error(`Failed to parse CSV file: ${error.message}`);
    }
}

async function writeErrorsToFile(failedItems, originalFilePath) {
    const errorFilePath = `${originalFilePath}.errors.csv`;
    const header = 'url,prefix\n';
    const content = failedItems.map(item => {
        const prefix = item.prefix ? `"${item.prefix}"` : '';
        return `"${item.url}",${prefix}`;
    }).join('\n');
    
    fs.writeFileSync(errorFilePath, header + content, 'utf-8');
    console.log(`\nFailed entries have been written to: ${errorFilePath}`);
}

async function processVideo(url, prefix, browser) {
    const page = await browser.newPage();
    
    try {
        // Navigate to the page
        await page.goto(url, { waitUntil: 'networkidle0' });
        
        // Get the page title
        const title = await page.title();
        
        // Wait for the video element
        await page.waitForSelector('#player_el');
        
        // Get video source
        const videoSrc = await page.$eval('#player_el', el => el.src);
        
        if (!videoSrc) {
            throw new Error('Could not find video source');
        }

        // Get canonical URL
        const canonicalUrl = await getCanonicalUrl(videoSrc);
        
        // Process the title with optional prefix
        const processedTitle = await processTitle(title, prefix);
        
        // Create output filename
        const outputPath = path.join(process.cwd(), `${processedTitle}.mp4`);
        
        console.log('\nProcessing video...');
        console.log('Title:', processedTitle);
        console.log('Source:', canonicalUrl);
        console.log('Output:', outputPath);
        
        // Download the video
        await downloadVideo(canonicalUrl, outputPath);
        
        console.log('Download complete!');
        
    } finally {
        await page.close();
    }
}

async function main() {
    const args = getArgs();
    
    if (!args.mode || args.mode === 'url' && !args.url || args.url === '-h' || args.url === '--help') {
        showHelp(args.isPackaged);
    }
    
    const browser = await getBrowser();
    
    try {
        if (args.mode === 'file') {
            const urlList = await processUrlList(args.filePath);
            console.log(`Found ${urlList.length} URLs to process`);
            
            const failedItems = [];
            
            for (const [index, item] of urlList.entries()) {
                console.log(`\nProcessing item ${index + 1}/${urlList.length}`);
                console.log(`URL: ${item.url}`);
                if (item.prefix) console.log(`Prefix: ${item.prefix}`);
                
                try {
                    await processVideo(item.url, item.prefix, browser);
                } catch (error) {
                    console.error(`Error processing ${item.url}:`, error.message);
                    failedItems.push({
                        url: item.url,
                        prefix: item.prefix,
                        error: error.message
                    });
                    continue;
                }
            }
            
            if (failedItems.length > 0) {
                console.log(`\n${failedItems.length} items failed to process`);
                await writeErrorsToFile(failedItems, args.filePath);
            }
        } else {
            await processVideo(args.url, args.prefix, browser);
        }
    } finally {
        await browser.close();
    }
}

// Run the script
main().catch(console.error);
