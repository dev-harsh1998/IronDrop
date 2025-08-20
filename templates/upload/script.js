// SPDX-License-Identifier: MIT
/**
 * IronDrop Upload Interface JavaScript
 * Handles drag-and-drop, file uploads, and progress tracking
 */

class UploadManager {
    constructor() {
        this.files = new Map(); // file-id -> file info
        this.uploads = new Map(); // file-id -> XMLHttpRequest
        this.fileCounter = 0;
        this.totalBytes = 0;
        this.uploadedBytes = 0;

        this.init();
    }

    init() {
        this.setupElements();
        this.setupEventListeners();
        this.updateSummary();
    }

    setupElements() {
        this.dropZone = document.getElementById('dropZone');
        this.fileInput = document.getElementById('fileInput');
        this.browseButton = document.getElementById('browseButton');
        this.uploadQueue = document.getElementById('uploadQueue');
        this.queueList = document.getElementById('queueList');
        this.uploadSummary = document.getElementById('uploadSummary');
        this.uploadMessages = document.getElementById('uploadMessages');
        this.clearCompletedBtn = document.getElementById('clearCompleted');
        this.cancelAllBtn = document.getElementById('cancelAll');

        // Summary elements
        this.totalFilesEl = document.getElementById('totalFiles');
        this.totalSizeEl = document.getElementById('totalSize');
        this.completedFilesEl = document.getElementById('completedFiles');
        this.totalProgressEl = document.getElementById('totalProgress');
        this.progressTextEl = document.getElementById('progressText');
    }

    setupEventListeners() {
        // Drag and drop events
        this.dropZone.addEventListener('click', () => this.fileInput.click());
        this.browseButton.addEventListener('click', (e) => {
            e.stopPropagation();
            this.fileInput.click();
        });

        this.fileInput.addEventListener('change', (e) => {
            this.handleFiles(Array.from(e.target.files));
        });

        // Drag events
        this.dropZone.addEventListener('dragover', this.handleDragOver.bind(this));
        this.dropZone.addEventListener('dragleave', this.handleDragLeave.bind(this));
        this.dropZone.addEventListener('drop', this.handleDrop.bind(this));

        // Touch events for mobile devices
        this.dropZone.addEventListener('touchstart', this.handleTouchStart.bind(this), { passive: false });
        this.dropZone.addEventListener('touchmove', this.handleTouchMove.bind(this), { passive: false });
        this.dropZone.addEventListener('touchend', this.handleTouchEnd.bind(this), { passive: false });

        // Prevent default drag behaviors on the document
        ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
            document.addEventListener(eventName, this.preventDefaults.bind(this), false);
        });

        // Queue actions
        this.clearCompletedBtn.addEventListener('click', this.clearCompleted.bind(this));
        this.cancelAllBtn.addEventListener('click', this.cancelAll.bind(this));

        // Prevent page reload on file drop outside drop zone
        document.addEventListener('drop', this.preventDefaults.bind(this), false);
    }

    preventDefaults(e) {
        e.preventDefault();
        e.stopPropagation();
    }

    handleDragOver(e) {
        this.preventDefaults(e);
        this.dropZone.classList.add('drag-over');
    }

    handleDragLeave(e) {
        this.preventDefaults(e);
        // Only remove drag-over if we're actually leaving the drop zone
        if (!this.dropZone.contains(e.relatedTarget)) {
            this.dropZone.classList.remove('drag-over');
        }
    }

    handleDrop(e) {
        this.preventDefaults(e);
        this.dropZone.classList.remove('drag-over');

        const files = Array.from(e.dataTransfer.files);
        this.handleFiles(files);
    }

    // Touch event handlers for mobile devices
    handleTouchStart(e) {
        // Provide visual feedback on touch
        this.dropZone.classList.add('touch-active');
    }

    handleTouchMove(e) {
        this.preventDefaults(e);
    }

    handleTouchEnd(e) {
        this.preventDefaults(e);
        this.dropZone.classList.remove('touch-active');

        // If touch ends on the drop zone, open file picker
        if (e.target === this.dropZone || this.dropZone.contains(e.target)) {
            this.fileInput.click();
        }
    }

    handleFiles(fileList) {
        if (fileList.length === 0) return;

        const validFiles = [];
        const errors = [];

        fileList.forEach(file => {
            const validation = this.validateFile(file);
            if (validation.valid) {
                validFiles.push(file);
            } else {
                errors.push(validation.error);
            }
        });

        // Show validation errors
        if (errors.length > 0) {
            this.showMessage('warning', 'File Validation Issues',
                errors.slice(0, 3).join(', ') +
                (errors.length > 3 ? ` and ${errors.length - 3} more files` : ''));
        }

        // Add valid files to queue
        if (validFiles.length > 0) {
            validFiles.forEach(file => this.addFileToQueue(file));
            this.startUploads();
        }
    }

    validateFile(file) {
        // No size limit - direct streaming handles any file size efficiently
        return { valid: true };
    }

    addFileToQueue(file) {
        const fileId = `file-${++this.fileCounter}`;
        const fileInfo = {
            id: fileId,
            file: file,
            status: 'pending', // pending, uploading, completed, error
            progress: 0,
            uploadedBytes: 0,
            error: null
        };

        this.files.set(fileId, fileInfo);
        this.totalBytes += file.size;

        this.renderQueueItem(fileInfo);
        this.updateSummary();
        this.showQueue();
    }

    renderQueueItem(fileInfo) {
        const item = document.createElement('div');
        item.className = 'queue-item';
        item.id = fileInfo.id;
        item.innerHTML = this.getQueueItemHTML(fileInfo);

        this.queueList.appendChild(item);

        // Add remove button handler
        const removeBtn = item.querySelector('.file-action.remove');
        if (removeBtn) {
            removeBtn.addEventListener('click', () => this.removeFile(fileInfo.id));
        }
    }

    getQueueItemHTML(fileInfo) {
        const { file, status, progress, error } = fileInfo;
        const statusClass = status === 'error' ? 'error' :
            status === 'completed' ? 'completed' :
                status === 'uploading' ? 'uploading' : '';

        return `
            <div class="file-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                    <polyline points="14,2 14,8 20,8"/>
                    <line x1="16" y1="13" x2="8" y2="13"/>
                    <line x1="16" y1="17" x2="8" y2="17"/>
                    <line x1="10" y1="9" x2="8" y2="9"/>
                </svg>
            </div>
            <div class="file-details">
                <div class="file-name" title="${this.escapeHtml(file.name)}">${this.escapeHtml(file.name)}</div>
                <div class="file-meta">
                    <span class="file-size">${this.formatBytes(file.size)}</span>
                    <span class="file-status ${statusClass}">${this.getStatusText(status, error)}</span>
                </div>
            </div>
            <div class="file-progress" style="display: ${status === 'uploading' ? 'block' : 'none'}">
                <div class="progress-bar">
                    <div class="progress-fill" style="width: ${progress}%"></div>
                </div>
                <span class="progress-text">${Math.round(progress)}%</span>
            </div>
            <div class="file-actions">
                ${status === 'pending' ? `
                    <button class="file-action remove" title="Remove file">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <polyline points="3,6 5,6 21,6"/>
                            <path d="m19,6v14a2,2 0 0,1 -2,2H7a2,2 0 0,1 -2,-2V6m3,0V4a2,2 0 0,1 2,-2h4a2,2 0 0,1 2,2v2"/>
                        </svg>
                    </button>
                ` : ''}
            </div>
        `;
    }

    getStatusText(status, error) {
        switch (status) {
            case 'pending': return 'Pending';
            case 'uploading': return 'Uploading...';
            case 'completed': return 'Completed';
            case 'error': return error || 'Error';
            default: return 'Unknown';
        }
    }

    updateQueueItem(fileInfo) {
        const item = document.getElementById(fileInfo.id);
        if (item) {
            item.innerHTML = this.getQueueItemHTML(fileInfo);
            item.className = `queue-item ${fileInfo.status}`;

            // Re-add remove button handler if needed
            if (fileInfo.status === 'pending') {
                const removeBtn = item.querySelector('.file-action.remove');
                if (removeBtn) {
                    removeBtn.addEventListener('click', () => this.removeFile(fileInfo.id));
                }
            }
        }
    }

    removeFile(fileId) {
        const fileInfo = this.files.get(fileId);
        if (!fileInfo) return;

        // Cancel upload if in progress
        if (fileInfo.status === 'uploading') {
            const xhr = this.uploads.get(fileId);
            if (xhr) {
                xhr.abort();
                this.uploads.delete(fileId);
            }
        }

        // Update total bytes
        this.totalBytes -= fileInfo.file.size;
        this.uploadedBytes -= fileInfo.uploadedBytes;

        // Remove from DOM and data structures
        const item = document.getElementById(fileId);
        if (item) item.remove();

        this.files.delete(fileId);

        this.updateSummary();

        // Hide queue if empty
        if (this.files.size === 0) {
            this.hideQueue();
        }
    }

    clearCompleted() {
        const completedFiles = Array.from(this.files.values())
            .filter(file => file.status === 'completed');

        completedFiles.forEach(file => this.removeFile(file.id));
    }

    cancelAll() {
        const allFiles = Array.from(this.files.keys());
        allFiles.forEach(fileId => this.removeFile(fileId));
    }

    async startUploads() {
        const pendingFiles = Array.from(this.files.values())
            .filter(file => file.status === 'pending');

        // Start up to 3 concurrent uploads
        const maxConcurrent = 3;
        const uploading = Array.from(this.uploads.keys()).length;
        const toStart = Math.min(maxConcurrent - uploading, pendingFiles.length);

        for (let i = 0; i < toStart; i++) {
            this.uploadFile(pendingFiles[i]);
        }
    }

    async uploadFile(fileInfo) {
        const { id, file } = fileInfo;

        // Update status
        fileInfo.status = 'uploading';
        this.updateQueueItem(fileInfo);

        // Get current path from URL or default to /
        const currentPath = this.getCurrentPath();

        // Create XMLHttpRequest for progress tracking
        const xhr = new XMLHttpRequest();
        this.uploads.set(id, xhr);

        // Progress handler
        xhr.upload.addEventListener('progress', (e) => {
            if (e.lengthComputable) {
                const progress = (e.loaded / e.total) * 100;
                fileInfo.progress = progress;
                fileInfo.uploadedBytes = e.loaded;
                this.updateQueueItem(fileInfo);
                this.updateSummary();
            }
        });

        // Completion handlers
        xhr.addEventListener('load', () => {
            this.uploads.delete(id);

            if (xhr.status >= 200 && xhr.status < 300) {
                fileInfo.status = 'completed';
                fileInfo.progress = 100;
                this.updateQueueItem(fileInfo);
                this.showMessage('success', 'Upload Complete',
                    `Successfully uploaded ${file.name}`);
            } else {
                fileInfo.status = 'error';
                fileInfo.error = `Server error: ${xhr.status}`;
                this.updateQueueItem(fileInfo);
                this.showMessage('error', 'Upload Failed',
                    `Failed to upload ${file.name}: ${xhr.statusText}`);
            }

            this.updateSummary();
            this.startUploads(); // Start next upload
        });

        xhr.addEventListener('error', () => {
            this.uploads.delete(id);
            fileInfo.status = 'error';
            fileInfo.error = 'Network error';
            this.updateQueueItem(fileInfo);
            this.showMessage('error', 'Upload Failed',
                `Network error uploading ${file.name}`);
            this.updateSummary();
            this.startUploads(); // Start next upload
        });

        xhr.addEventListener('abort', () => {
            this.uploads.delete(id);
            // Don't update status here as file might be removed
        });

        // Send request with raw binary data
        const uploadPath = this.getUploadPath();
        xhr.open('POST', uploadPath);
        
        // Set headers for direct binary upload
        xhr.setRequestHeader('Content-Type', 'application/octet-stream');
        xhr.setRequestHeader('X-Filename', file.name);

        // Send raw file data instead of FormData
        xhr.send(file);
    }

    getCurrentPath() {
        // Extract path from URL or use root
        const path = window.location.pathname;
        return path.endsWith('/') ? path : path + '/';
    }

    getUploadPath() {
        // Get upload_to parameter from current URL
        const urlParams = new URLSearchParams(window.location.search);
        const uploadTo = urlParams.get('upload_to');

        if (uploadTo) {
            return `/_irondrop/upload?upload_to=${encodeURIComponent(uploadTo)}`;
        } else {
            return '/_irondrop/upload';
        }
    }

    updateSummary() {
        const totalFiles = this.files.size;
        const completedFiles = Array.from(this.files.values())
            .filter(file => file.status === 'completed').length;

        // Calculate total progress
        let totalProgress = 0;
        if (this.totalBytes > 0) {
            const currentUploadedBytes = Array.from(this.files.values())
                .reduce((sum, file) => sum + file.uploadedBytes, 0);
            totalProgress = (currentUploadedBytes / this.totalBytes) * 100;
        }

        // Update DOM
        this.totalFilesEl.textContent = totalFiles;
        this.totalSizeEl.textContent = this.formatBytes(this.totalBytes);
        this.completedFilesEl.textContent = completedFiles;
        this.totalProgressEl.style.width = `${totalProgress}%`;
        this.progressTextEl.textContent = `${Math.round(totalProgress)}% complete`;

        // Show/hide summary
        if (totalFiles > 0) {
            this.uploadSummary.style.display = 'block';
        } else {
            this.uploadSummary.style.display = 'none';
        }
    }

    showQueue() {
        this.uploadQueue.style.display = 'block';
    }

    hideQueue() {
        this.uploadQueue.style.display = 'none';
    }

    showMessage(type, title, message) {
        const messageEl = document.createElement('div');
        messageEl.className = `upload-message ${type}`;

        const iconSvg = type === 'success' ?
            '<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22,4 12,14.01 9,11.01"/>' :
            type === 'error' ?
                '<circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>' :
                '<path d="m21.73,18-8-14a2,2,0,0,0-3.48,0l-8,14A2,2,0,0,0,4,21H20A2,2,0,0,0,21.73,18Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>';

        messageEl.innerHTML = `
            <div class="message-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    ${iconSvg}
                </svg>
            </div>
            <div class="message-content">
                <div class="message-title">${this.escapeHtml(title)}</div>
                <div class="message-text">${this.escapeHtml(message)}</div>
            </div>
        `;

        this.uploadMessages.appendChild(messageEl);

        // Auto-remove after 5 seconds
        setTimeout(() => {
            if (messageEl.parentNode) {
                messageEl.remove();
            }
        }, 5000);
    }

    formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// Inline Upload Form Handler
class InlineUploadForm {
    constructor() {
        this.init();
    }

    init() {
        const uploadForm = document.querySelector('.inline-upload');
        if (!uploadForm) return;

        this.setupToggle(uploadForm);
        this.setupForm(uploadForm);
    }

    setupToggle(uploadForm) {
        const toggle = uploadForm.querySelector('.upload-toggle');
        if (toggle) {
            toggle.addEventListener('click', () => {
                uploadForm.classList.toggle('collapsed');
            });
        }
    }

    setupForm(uploadForm) {
        const form = uploadForm.querySelector('form');
        const fileInput = uploadForm.querySelector('input[type="file"]');
        const submitBtn = uploadForm.querySelector('.upload-button');

        if (!form || !fileInput || !submitBtn) return;

        fileInput.addEventListener('change', () => {
            submitBtn.disabled = fileInput.files.length === 0;
        });

        form.addEventListener('submit', async (e) => {
            e.preventDefault();

            if (fileInput.files.length === 0) return;

            submitBtn.disabled = true;
            submitBtn.textContent = 'Uploading...';

            const formData = new FormData();
            Array.from(fileInput.files).forEach(file => {
                formData.append('file', file);
            });

            try {
                const response = await fetch(window.location.pathname + '?upload=true', {
                    method: 'POST',
                    body: formData
                });

                if (response.ok) {
                    // Reload page to show new files
                    window.location.reload();
                } else {
                    throw new Error(`Upload failed: ${response.statusText}`);
                }
            } catch (error) {
                console.error('Upload error:', error);
                submitBtn.textContent = 'Upload Failed';
                setTimeout(() => {
                    submitBtn.textContent = 'Upload Files';
                    submitBtn.disabled = false;
                }, 2000);
            }
        });
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    // Initialize upload manager if on upload page
    if (document.getElementById('dropZone')) {
        new UploadManager();
    }

    // Initialize inline upload form
    new InlineUploadForm();
});