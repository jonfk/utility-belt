const puppeteer = require('puppeteer-core');
const chrome = require('chrome-launcher');
const sanitize = require('sanitize-filename');
const moment = require('moment');
const axios = require('axios');
const fs = require('fs');
const path = require('path');

async function getBrowser() {
    const chromePath = await chrome.getChromePath();
    return puppeteer.launch({
        executablePath: chromePath,
        headless: "new"
    });
}

async function processTitle(title, prefix = '') {
    // Add prefix if provided
    if (prefix) {
        title = `${prefix} ${title}`;
    }
    
    // Remove URLs
    title = title.replace(/(?:https?|ftp):\/\/[\n\S]+/g, '');
    
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
    return { args, isPackaged };
}

function showHelp(isPackaged) {
    const command = isPackaged ? 'video-downloader' : 'node script.js';
    console.log(`
Usage: ${command} [URL] [prefix]
    URL     The video page URL to download from
    prefix  Optional text to prepend to the filename

Options:
    -h, --help  Show this help message

Example:
    ${command} https://example.com/video "My Video"
`);
    process.exit(0);
}

async function main() {
    const { args, isPackaged } = getArgs();
    
    if (args.length < 1 || args[0] === '-h' || args[0] === '--help') {
        showHelp(isPackaged);
    }
    
    const url = args[0];
    const prefix = args[1] || '';

    console.log(`url:${url}\nprefix: ${prefix}`);
    const browser = await getBrowser();
    
    try {
        const page = await browser.newPage();
        
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
        
        console.log('Downloading video...');
        console.log('Title:', processedTitle);
        console.log('Source:', canonicalUrl);
        console.log('Output:', outputPath);
        
        // Download the video
        await downloadVideo(canonicalUrl, outputPath);
        
        console.log('Download complete!');
        
    } catch (error) {
        console.error('An error occurred:', error.message);
        process.exit(1);
    } finally {
        await browser.close();
    }
}

// Run the script
main().catch(console.error);
