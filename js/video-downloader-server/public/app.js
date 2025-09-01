function videoDownloader() {
  return {
    // Form data
    nameForm: {
      url: '',
      result: '',
      loading: false,
      error: ''
    },
    
    downloadForm: {
      url: '',
      name: '',
      loading: false,
      error: '',
      success: ''
    },
    
    // Progress data
    activeDownloads: [],
    completedDownloads: [],
    completedLoading: false,
    
    // Polling
    progressInterval: null,
    
    init() {
      this.loadCompleted();
      this.startProgressPolling();
    },
    
    destroy() {
      if (this.progressInterval) {
        clearInterval(this.progressInterval);
      }
    },
    
    async resolveName() {
      if (!this.nameForm.url) return;
      
      this.nameForm.loading = true;
      this.nameForm.error = '';
      this.nameForm.result = '';
      
      try {
        const response = await fetch('/api/name', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({
            url: this.nameForm.url
          })
        });
        
        if (!response.ok) {
          const error = await response.json();
          throw new Error(error.message || 'Failed to resolve name');
        }
        
        const data = await response.json();
        this.nameForm.result = data.name;
        
      } catch (error) {
        this.nameForm.error = error.message;
      } finally {
        this.nameForm.loading = false;
      }
    },
    
    copyFromResolve() {
      if (this.nameForm.result) {
        this.downloadForm.name = this.nameForm.result;
        this.downloadForm.url = this.nameForm.url;
      }
    },
    
    async startDownload() {
      if (!this.downloadForm.url || !this.downloadForm.name) return;
      
      this.downloadForm.loading = true;
      this.downloadForm.error = '';
      this.downloadForm.success = '';
      
      try {
        const response = await fetch('/api/download', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({
            url: this.downloadForm.url,
            name: this.downloadForm.name
          })
        });
        
        if (!response.ok) {
          const error = await response.json();
          throw new Error(error.message || 'Failed to start download');
        }
        
        const data = await response.json();
        this.downloadForm.success = `Download started! Job ID: ${data.jobId}`;
        
        // Clear form after successful submission
        setTimeout(() => {
          this.downloadForm.url = '';
          this.downloadForm.name = '';
          this.downloadForm.success = '';
        }, 3000);
        
        // Immediately poll for progress to show the new download
        this.pollProgress();
        
      } catch (error) {
        this.downloadForm.error = error.message;
      } finally {
        this.downloadForm.loading = false;
      }
    },
    
    async pollProgress() {
      try {
        const response = await fetch('/api/downloads/progress');
        
        if (!response.ok) {
          console.error('Failed to fetch progress');
          return;
        }
        
        const data = await response.json();
        this.activeDownloads = data.downloads || [];
        
      } catch (error) {
        console.error('Error polling progress:', error);
      }
    },
    
    startProgressPolling() {
      // Poll every 2 seconds
      this.progressInterval = setInterval(() => {
        this.pollProgress();
      }, 2000);
    },
    
    async loadCompleted() {
      this.completedLoading = true;
      
      try {
        const response = await fetch('/api/downloads/completed');
        
        if (!response.ok) {
          throw new Error('Failed to load completed downloads');
        }
        
        const data = await response.json();
        this.completedDownloads = data.downloads || [];
        
      } catch (error) {
        console.error('Error loading completed downloads:', error);
      } finally {
        this.completedLoading = false;
      }
    },
    
    formatBytes(bytes) {
      if (!bytes || bytes === 0) return '0 B';
      
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      
      return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    },
    
    formatDate(dateString) {
      if (!dateString) return '';
      
      try {
        const date = new Date(dateString);
        return date.toLocaleString();
      } catch (error) {
        return dateString;
      }
    }
  };
}

// Clean up on page unload
window.addEventListener('beforeunload', () => {
  // Alpine.js will handle cleanup automatically, but we can add custom cleanup here if needed
});