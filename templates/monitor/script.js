// SPDX-License-Identifier: MIT
// Monitor Page JavaScript - Real-time metrics with Chart.js support

let chartJsLoaded = false;
let chartsInitialized = false;

// Check if Chart.js loaded correctly
window.addEventListener('load', function() {
  chartJsLoaded = typeof Chart !== 'undefined';
  if (!chartJsLoaded) updateStatus('error', 'Error: Chart.js failed to load');
});

// Historical data storage - will be populated over time
const historyData = {
  timestamps: [],
  requests: {
    total: [],
    successful: [],
    errors: []
  },
  memory: {
    current: [],
    peak: []
  },
  uploads: {
    total: [],
    successful: [],
    failed: [],
    bytes: []
  },
  transfer: {
    uploadedMB: [],
    downloadedMB: []
  }
};

// Maximum number of data points to keep in history
const MAX_HISTORY_POINTS = 20;

// Chart instances
let requestsChart, memoryChart, dataChart;

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
    if (!statusEl) return;
    
    // Clear existing status classes
    statusEl.className = 'monitor-status';
    
    // Add new status class
    statusEl.classList.add(`status-${type}`);
    
    // Update text
    statusEl.textContent = message;
}

function setLoadingState(isLoading) {
    const monitorGrid = document.querySelector('.monitor-grid');
    const statusEl = document.getElementById('status');
    
    if (monitorGrid && isLoading) {
        monitorGrid.classList.add('loading-data');
    } else if (monitorGrid) {
        monitorGrid.classList.remove('loading-data');
    }
    
    if (statusEl && isLoading) {
        statusEl.classList.add('refreshing');
    } else if (statusEl) {
        statusEl.classList.remove('refreshing');
    }
}

// Update charts with new data
function updateCharts() {
    if (!historyData.timestamps.length) {
        return;
    }
    
    if (!requestsChart || !memoryChart || !dataChart) { return; }
    
    // Hide all fallback texts once charts are working
    document.querySelectorAll('.chart-fallback').forEach(el => { el.style.display = 'none'; });
    
    // Format time labels for display
    const timeLabels = historyData.timestamps.map(time => {
        const date = new Date(time);
        return date.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit', second:'2-digit'});
    });
    
    // Update Requests Chart
    requestsChart.data.labels = timeLabels;
    requestsChart.data.datasets[0].data = historyData.requests.total;
    requestsChart.data.datasets[1].data = historyData.requests.successful;
    requestsChart.data.datasets[2].data = historyData.requests.errors;
    requestsChart.update();
    
    // Update Memory Chart if available
    if (historyData.memory.current.some(val => val !== null)) {
        memoryChart.data.labels = timeLabels;
        memoryChart.data.datasets[0].data = historyData.memory.current;
        memoryChart.data.datasets[1].data = historyData.memory.peak;
        memoryChart.update();
    }
    
    // Update Data Transferred Chart
    dataChart.data.labels = timeLabels;
    dataChart.data.datasets[0].data = historyData.transfer.downloadedMB;
    dataChart.data.datasets[1].data = historyData.transfer.uploadedMB;
    dataChart.update();
}

// Add new data points to history
function addToHistory(data) {
    // Add timestamp
    historyData.timestamps.push(new Date().getTime());
    
    // Add requests data
    historyData.requests.total.push(data.requests.total);
    historyData.requests.successful.push(data.requests.successful);
    historyData.requests.errors.push(data.requests.errors);
    
    // Add memory data if available
    if (data.memory && data.memory.available) {
        historyData.memory.current.push(data.memory.current_mb);
        historyData.memory.peak.push(data.memory.peak_mb);
    } else {
        historyData.memory.current.push(null);
        historyData.memory.peak.push(null);
    }
    
    // Add upload data
    historyData.uploads.total.push(data.uploads.total_uploads);
    historyData.uploads.successful.push(data.uploads.successful_uploads);
    historyData.uploads.failed.push(data.uploads.failed_uploads);
    historyData.uploads.bytes.push(data.uploads.upload_bytes);
    // Add transfer series (MB)
    historyData.transfer.uploadedMB.push((data.uploads.upload_bytes || 0) / 1024 / 1024);
    historyData.transfer.downloadedMB.push((data.downloads.bytes_served || 0) / 1024 / 1024);
    
    // Trim history if needed
    if (historyData.timestamps.length > MAX_HISTORY_POINTS) {
        historyData.timestamps.shift();
        historyData.requests.total.shift();
        historyData.requests.successful.shift();
        historyData.requests.errors.shift();
        historyData.memory.current.shift();
        historyData.memory.peak.shift();
        historyData.uploads.total.shift();
        historyData.uploads.successful.shift();
        historyData.uploads.failed.shift();
        historyData.uploads.bytes.shift();
        historyData.transfer.uploadedMB.shift();
        historyData.transfer.downloadedMB.shift();
    }
}

// Data loading and updating
async function loadMetrics() {
    setLoadingState(true);

    try {
        const response = await fetch('/monitor?json=1');

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const rawText = await response.text();
        
        // Parse the JSON
        let data;
        try {
            data = JSON.parse(rawText);
        } catch (parseErr) {
            throw new Error("Failed to parse server response: " + parseErr.message);
        }
        
        updateMetrics(data);
        
        if (chartJsLoaded) {
            try {
                // Ensure charts are initialized
                if (!chartsInitialized && typeof initCharts === 'function') {
                    chartsInitialized = initCharts();
                }
                
                // Add data to history and update charts
                addToHistory(data);
                updateCharts();
            } catch (chartErr) {}
        } else {}
        
        // Update status
        updateStatus('ok', 'Connected');

    } catch (error) {
        updateStatus('error', `Error: ${error.message}`);
        clearMetrics();
    } finally {
        setLoadingState(false);
        updateTimestamp();
    }
}

function updateMetrics(data) {
    const { requests: r, uploads: u, downloads: d, memory: m } = data;

    // Helper function to safely update element text content
    function safeSetText(elementId, value) {
        const element = document.getElementById(elementId);
        if (element) {
            element.textContent = value;
        }
    }

    // Request metrics (dropdown-only now)
    const successRate = r.total ? ((r.successful / r.total) * 100).toFixed(2) + '%' : '0%';

    // Request dropdown metrics
    safeSetText('req_total_dd', r.total);
    safeSetText('req_success_dd', r.successful);
    safeSetText('req_errors_dd', r.errors);
    safeSetText('req_success_rate_dd', successRate);

    // Download metrics
    safeSetText('bytes_served', d.bytes_served.toLocaleString());
    safeSetText('mb_served', (d.bytes_served / 1024 / 1024).toFixed(2));

    // Upload metrics
    safeSetText('up_total', u.total_uploads);
    safeSetText('up_success', u.successful_uploads);
    safeSetText('up_failed', u.failed_uploads);
    safeSetText('up_mb', (u.upload_bytes / 1024 / 1024).toFixed(2));
    safeSetText('avg_file_size', humanBytes(u.average_upload_size));
    safeSetText('largest_upload', humanBytes(u.largest_upload));
    

    const successRateUpload = u.success_rate;
    const successRateUploadText = (successRateUpload && successRateUpload.toFixed) ?
        successRateUpload.toFixed(2) + '%' : successRateUpload + '%';
    safeSetText('upload_success_rate', successRateUploadText);

    // Uptime metrics (pretty only)
    safeSetText('uptime_pretty', prettyUptime(data.uptime_secs));

    // Memory dropdown metrics
    if (m) {
        safeSetText('mem_available', m.available ? 'Yes' : 'No');
        if (m.available) {
            const curMb = typeof m.current_mb === 'number' ? m.current_mb : (m.current_bytes ? (m.current_bytes / 1024 / 1024) : 0);
            const peakMb = typeof m.peak_mb === 'number' ? m.peak_mb : (m.peak_bytes ? (m.peak_bytes / 1024 / 1024) : 0);
            safeSetText('mem_current_mb', curMb.toFixed ? curMb.toFixed(2) : curMb);
            safeSetText('mem_peak_mb', peakMb.toFixed ? peakMb.toFixed(2) : peakMb);
        } else {
            safeSetText('mem_current_mb', '-');
            safeSetText('mem_peak_mb', '-');
        }
    }
}

function clearMetrics() {
    // Clear all metric values on error
    const metricElements = [
        'req_total', 'req_success', 'req_errors', 'req_success_rate',
        'bytes_served', 'mb_served',
        'up_total', 'up_success', 'up_failed', 'up_mb', 'avg_file_size', 'largest_upload',
        'upload_success_rate',
        'uptime_pretty'
    ];

    metricElements.forEach(id => {
        const el = document.getElementById(id);
        if (el) {
            el.textContent = '-';
        } else {
            console.warn(`Element with ID '${id}' not found during clearMetrics`);
        }
    });
}

function updateTimestamp() {
    const now = new Date();
    const timeString = now.toLocaleTimeString();
    const timestampEl = document.getElementById('last_updated');
    if (timestampEl) {
        timestampEl.textContent = timeString;
    } else {
        console.warn("Element with ID 'last_updated' not found");
    }
}

// Initialize charts
function initCharts() {
    try {
        console.log("Initializing charts...");
        
        if (typeof Chart !== 'function') {
            console.error("Chart constructor not available");
            // Show error in all chart fallbacks
            document.querySelectorAll('.chart-fallback').forEach(el => {
                el.textContent = 'Chart.js failed to load';
                el.style.color = '#f87171';
            });
            return false;
        }
        
        // Check if chart elements exist
        const requestsElement = document.getElementById('requestsChart');
        const memoryElement = document.getElementById('memoryChart');
        const dataElement = document.getElementById('dataChart');
        
        if (!requestsElement || !memoryElement || !dataElement) {
            console.error("Chart elements not found in DOM:", { 
                requests: !!requestsElement, 
                memory: !!memoryElement, 
                data: !!dataElement 
            });
            return false;
        }
        
        // Common chart options
        const commonOptions = {
            responsive: true,
            maintainAspectRatio: false,
            animation: {
                duration: 600
            },
            elements: {
                point: {
                    radius: 3,
                    hoverRadius: 5
                },
                line: {
                    tension: 0.3
                }
            },
            plugins: {
                legend: {
                    position: 'top',
                    labels: {
                        color: '#e5e5e5',
                        font: {
                            family: "'Inter', sans-serif",
                            size: 12
                        }
                    }
                },
                tooltip: {
                    backgroundColor: '#1a1a1a',
                    borderColor: 'rgba(64, 64, 64, 0.4)',
                    borderWidth: 1,
                    titleFont: {
                        family: "'Inter', sans-serif",
                        size: 12,
                        weight: 'normal'
                    },
                    bodyFont: {
                        family: "'Inter', sans-serif",
                        size: 12
                    },
                    padding: 10,
                    boxPadding: 4
                }
            },
            scales: {
                x: {
                    grid: {
                        color: 'rgba(255, 255, 255, 0.05)',
                        borderColor: 'rgba(255, 255, 255, 0.1)'
                    },
                    ticks: {
                        color: '#b0b0b0',
                        font: {
                            family: "'Inter', sans-serif",
                            size: 10
                        },
                        maxRotation: 0,
                        autoSkipPadding: 20
                    }
                },
                y: {
                    beginAtZero: true,
                    grid: {
                        color: 'rgba(255, 255, 255, 0.05)',
                        borderColor: 'rgba(255, 255, 255, 0.1)'
                    },
                    ticks: {
                        color: '#b0b0b0',
                        font: {
                            family: "'Inter', sans-serif",
                            size: 10
                        }
                    }
                }
            }
        };
        
        // Requests Chart
        const requestsCtx = requestsElement.getContext('2d');
        requestsChart = new Chart(requestsCtx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [
                    {
                        label: 'Total',
                        data: [],
                        borderColor: '#9CC8FF',
                        backgroundColor: 'rgba(156, 200, 255, 0.1)',
                        fill: true
                    },
                    {
                        label: 'Successful',
                        data: [],
                        borderColor: '#4ADE80',
                        backgroundColor: 'rgba(74, 222, 128, 0.1)',
                        fill: true
                    },
                    {
                        label: 'Errors',
                        data: [],
                        borderColor: '#F87171',
                        backgroundColor: 'rgba(248, 113, 113, 0.1)',
                        fill: true
                    }
                ]
            },
            options: commonOptions
        });
        
        // Memory Usage Chart
        const memoryCtx = memoryElement.getContext('2d');
        memoryChart = new Chart(memoryCtx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [
                    {
                        label: 'Current (MB)',
                        data: [],
                        borderColor: '#60A5FA',
                        backgroundColor: 'rgba(96, 165, 250, 0.1)',
                        fill: true
                    },
                    {
                        label: 'Peak (MB)',
                        data: [],
                        borderColor: '#F59E0B',
                        backgroundColor: 'rgba(245, 158, 11, 0.1)',
                        fill: true
                    }
                ]
            },
            options: commonOptions
        });
        
        // Data Transferred Chart
        const dataCtx = dataElement.getContext('2d');
        dataChart = new Chart(dataCtx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [
                    {
                        label: 'Downloaded (MB)',
                        data: [],
                        borderColor: '#60A5FA',
                        backgroundColor: 'rgba(96, 165, 250, 0.1)',
                        fill: true
                    },
                    {
                        label: 'Uploaded (MB)',
                        data: [],
                        borderColor: '#A78BFA',
                        backgroundColor: 'rgba(167, 139, 250, 0.1)',
                        fill: true
                    }
                ]
            },
            options: commonOptions
        });
        
        console.log("Charts initialized successfully");
        return true;
    } catch (err) {
        console.error("Error initializing charts:", err);
        return false;
    }
}

// Initialize and start auto-refresh
function init() {
    // Check if we have the status element structure we expect
    const statusEl = document.getElementById('status');
    if (!statusEl) {
        console.error("Status element not found");
    } else {
        // Make sure status element has content
        if (!statusEl.textContent) {
            statusEl.textContent = 'Initializing...';
        }
    }
    
    // Check if chart container exists
    const chartsContainer = document.getElementById('chartsContainer');
    // chartsContainer presence not critical for init logging
    
    // Try to initialize charts if Chart.js is available
    if (typeof Chart !== 'undefined') {
        chartJsLoaded = true;
        chartsInitialized = initCharts();
        console.log("Charts initialization result:", chartsInitialized);
    } else {
        console.warn("Chart.js not available during initialization");
        chartJsLoaded = false;
    }
    
    // Initial load
    loadMetrics();

    // Auto-refresh every 4 seconds
    setInterval(loadMetrics, 4000);

    // Add manual refresh capability (optional)
    document.addEventListener('keydown', (e) => {
        if (e.key === 'r' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            loadMetrics();
        }
    });
    
    console.log("Monitor initialization complete");
}

// Memory cleanup functionality
async function performMemoryCleanup() {
    const button = document.getElementById('cleanup_memory_btn');
    const status = document.getElementById('cleanup_status');
    
    if (!button || !status) {
        console.warn('Memory cleanup button or status element not found');
        return;
    }
    
    // Disable button and show loading state
    button.disabled = true;
    button.textContent = '🔄 Cleaning...';
    status.textContent = 'Performing memory cleanup...';
    status.className = 'cleanup-status loading';
    
    try {
        console.log('Initiating memory cleanup...');
        const response = await fetch('/_irondrop/cleanup-memory', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            }
        });
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        const result = await response.json();
        console.log('Memory cleanup result:', result);
        
        if (result.status === 'success') {
            status.textContent = `✅ ${result.message} (${result.cleanup_time_ms}ms)`;
            status.className = 'cleanup-status success';
            
            // Force a metrics reload to show updated memory usage
            setTimeout(() => {
                loadMetrics();
            }, 500);
        } else {
            status.textContent = `❌ ${result.message}`;
            status.className = 'cleanup-status error';
        }
        
    } catch (error) {
        console.error('Memory cleanup failed:', error);
        status.textContent = `❌ Cleanup failed: ${error.message}`;
        status.className = 'cleanup-status error';
    } finally {
        // Re-enable button after a delay
        setTimeout(() => {
            button.disabled = false;
            button.textContent = '🧹 Cleanup Memory';
            
            // Clear status message after 10 seconds
            setTimeout(() => {
                status.textContent = '';
                status.className = 'cleanup-status';
            }, 10000);
        }, 1000);
    }
}

// Start when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}

// Add event listener for memory cleanup button when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
    const cleanupButton = document.getElementById('cleanup_memory_btn');
    if (cleanupButton) {
        cleanupButton.addEventListener('click', performMemoryCleanup);
        
    } else {
        
    }

    // Memory dropdown toggle
    const toggle = document.getElementById('memory_dropdown_toggle');
    const content = document.getElementById('memory_dropdown_content');
    if (toggle && content) {
        toggle.addEventListener('click', () => {
            const expanded = toggle.getAttribute('aria-expanded') === 'true';
            toggle.setAttribute('aria-expanded', String(!expanded));
            if (expanded) {
                content.setAttribute('hidden', '');
                toggle.textContent = 'Memory Details ▾';
            } else {
                content.removeAttribute('hidden');
                toggle.textContent = 'Memory Details ▴';
            }
        });
    }

    // Requests dropdown toggle
    const rToggle = document.getElementById('requests_dropdown_toggle');
    const rContent = document.getElementById('requests_dropdown_content');
    if (rToggle && rContent) {
        rToggle.addEventListener('click', () => {
            const expanded = rToggle.getAttribute('aria-expanded') === 'true';
            rToggle.setAttribute('aria-expanded', String(!expanded));
            if (expanded) {
                rContent.setAttribute('hidden', '');
                rToggle.textContent = 'Request Details ▾';
            } else {
                rContent.removeAttribute('hidden');
                rToggle.textContent = 'Request Details ▴';
            }
        });
    }

    // Transfer dropdown toggle
    const tToggle = document.getElementById('transfer_dropdown_toggle');
    const tContent = document.getElementById('transfer_dropdown_content');
    if (tToggle && tContent) {
        tToggle.addEventListener('click', () => {
            const expanded = tToggle.getAttribute('aria-expanded') === 'true';
            tToggle.setAttribute('aria-expanded', String(!expanded));
            if (expanded) {
                tContent.setAttribute('hidden', '');
                tToggle.textContent = 'Transfer Details ▾';
            } else {
                tContent.removeAttribute('hidden');
                tToggle.textContent = 'Transfer Details ▴';
            }
        });
    }
});
