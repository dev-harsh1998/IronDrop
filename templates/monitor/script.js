// Monitor Page JavaScript - Real-time metrics

// Utility functions
function humanBytes(bytes) {
    if (bytes < 1024) return bytes + " B";
    const units = ['KB', 'MB', 'GB', 'TB'];
    let unitIndex = -1;

    do {
        bytes /= 1024;
        unitIndex++;
    } while (bytes >= 1024 && unitIndex < units.length - 1);

    return bytes.toFixed(2) + ' ' + units[unitIndex];
}

function prettyUptime(seconds) {
    const days = Math.floor(seconds / 86400);
    seconds %= 86400;
    const hours = Math.floor(seconds / 3600);
    seconds %= 3600;
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;

    let output = [];
    if (days) output.push(days + 'd');
    if (hours) output.push(hours + 'h');
    if (minutes) output.push(minutes + 'm');
    output.push(secs + 's');

    return output.join(' ');
}

// Status management
function updateStatus(type, message) {
    const statusEl = document.getElementById('status');
    const statusText = statusEl.querySelector('.status-text');

    // Clear existing status classes
    statusEl.className = 'status-indicator';

    // Add new status class
    statusEl.classList.add(`status-${type}`);

    // Update text
    statusText.textContent = message;
}

function setLoadingState(isLoading) {
    const metricsGrid = document.querySelector('.metrics-grid');
    if (isLoading) {
        metricsGrid.classList.add('loading-data');
        document.getElementById('status').classList.add('refreshing');
    } else {
        metricsGrid.classList.remove('loading-data');
        document.getElementById('status').classList.remove('refreshing');
    }
}

// Data loading and updating
async function loadMetrics() {
    setLoadingState(true);

    try {
        const response = await fetch('/_irondrop/monitor?json=1');

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const data = await response.json();
        updateMetrics(data);
        updateStatus('ok', 'Connected');

    } catch (error) {
        console.error('Failed to load metrics:', error);
        updateStatus('error', `Error: ${error.message}`);
        clearMetrics();
    } finally {
        setLoadingState(false);
        updateTimestamp();
    }
}

function updateMetrics(data) {
    const { requests: r, uploads: u, downloads: d } = data;

    // Request metrics
    document.getElementById('req_total').textContent = r.total;
    document.getElementById('req_success').textContent = r.successful;
    document.getElementById('req_errors').textContent = r.errors;

    const successRate = r.total ? ((r.successful / r.total) * 100).toFixed(2) + '%' : '0%';
    document.getElementById('req_success_rate').textContent = successRate;

    // Download metrics
    document.getElementById('bytes_served').textContent = d.bytes_served.toLocaleString();
    document.getElementById('mb_served').textContent = (d.bytes_served / 1024 / 1024).toFixed(2);

    // Upload metrics
    document.getElementById('up_total').textContent = u.total_uploads;
    document.getElementById('up_success').textContent = u.successful_uploads;
    document.getElementById('up_failed').textContent = u.failed_uploads;
    document.getElementById('files_uploaded').textContent = u.files_uploaded;
    document.getElementById('up_bytes').textContent = u.upload_bytes.toLocaleString();
    document.getElementById('up_mb').textContent = (u.upload_bytes / 1024 / 1024).toFixed(2);
    document.getElementById('avg_file_size').textContent = humanBytes(u.average_upload_size);
    document.getElementById('largest_upload').textContent = humanBytes(u.largest_upload);
    document.getElementById('concurrent_uploads').textContent = u.concurrent_uploads;

    // Handle potential undefined or non-numeric values
    const avgProcessing = u.average_processing_ms;
    document.getElementById('avg_processing').textContent =
        (avgProcessing && avgProcessing.toFixed) ? avgProcessing.toFixed(1) : avgProcessing;

    const successRateUpload = u.success_rate;
    document.getElementById('upload_success_rate').textContent =
        (successRateUpload && successRateUpload.toFixed) ?
            successRateUpload.toFixed(2) + '%' : successRateUpload + '%';

    // Uptime metrics
    document.getElementById('uptime_secs').textContent = data.uptime_secs;
    document.getElementById('uptime_pretty').textContent = prettyUptime(data.uptime_secs);
}

function clearMetrics() {
    // Clear all metric values on error
    const metricElements = [
        'req_total', 'req_success', 'req_errors', 'req_success_rate',
        'bytes_served', 'mb_served',
        'up_total', 'up_success', 'up_failed', 'files_uploaded',
        'up_bytes', 'up_mb', 'avg_file_size', 'largest_upload',
        'concurrent_uploads', 'avg_processing', 'upload_success_rate',
        'uptime_secs', 'uptime_pretty'
    ];

    metricElements.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = '-';
    });
}

function updateTimestamp() {
    const now = new Date();
    const timeString = now.toLocaleTimeString();
    document.getElementById('last_updated').textContent = timeString;
}

// Initialize and start auto-refresh
function init() {
    // Initial load
    loadMetrics();

    // Auto-refresh every 5 seconds
    setInterval(loadMetrics, 5000);

    // Add manual refresh capability (optional)
    document.addEventListener('keydown', (e) => {
        if (e.key === 'r' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            loadMetrics();
        }
    });
}

// Start when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
