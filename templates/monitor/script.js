// Monitor Page JavaScript - Real-time metrics with Chart.js support

// Debug variables to track chart status
let chartJsLoaded = false;
let chartsInitialized = false;
let dataPointsCollected = 0;

// Check if Chart.js loaded correctly
window.addEventListener('load', function() {
  if (typeof Chart === 'undefined') {
    console.error("Chart.js failed to load");
    updateStatus('error', 'Error: Chart.js failed to load');
    chartJsLoaded = false;
  } else {
    console.log("Chart.js loaded successfully");
    chartJsLoaded = true;
  }
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
  }
};

// Maximum number of data points to keep in history
const MAX_HISTORY_POINTS = 20;

// Chart instances
let requestsChart, memoryChart, uploadsChart;

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
    console.log("Updating charts with history length:", historyData.timestamps.length);
    dataPointsCollected = historyData.timestamps.length;
    
    if (!historyData.timestamps.length) {
        console.log("No history data yet, skipping chart update");
        return;
    }
    
    if (!requestsChart || !memoryChart || !uploadsChart) {
        console.error("Charts not initialized");
        return;
    }
    
    // Hide all fallback texts once charts are working
    document.querySelectorAll('.chart-fallback').forEach(el => {
        el.style.display = 'none';
    });
    
    console.log("Chart fallback messages hidden");
    
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
    
    // Update Uploads Chart
    uploadsChart.data.labels = timeLabels;
    uploadsChart.data.datasets[0].data = historyData.uploads.total;
    uploadsChart.data.datasets[1].data = historyData.uploads.successful;
    uploadsChart.data.datasets[2].data = historyData.uploads.failed;
    uploadsChart.data.datasets[3].data = historyData.uploads.bytes.map(b => b / 1024 / 1024);
    uploadsChart.update();
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
    }
}

// Data loading and updating
async function loadMetrics() {
    setLoadingState(true);

    try {
        // Log the current fetch URL for debugging
        console.log("Fetching metrics from: /monitor?json=1");
        const response = await fetch('/monitor?json=1');

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        // Debug the raw response
        const rawText = await response.text();
        console.log("Raw response:", rawText);
        
        // Parse the JSON (separate step for better debugging)
        let data;
        try {
            data = JSON.parse(rawText);
            console.log("Data parsed successfully:", data);
        } catch (parseErr) {
            console.error("JSON parse error:", parseErr);
            throw new Error("Failed to parse server response: " + parseErr.message);
        }
        
        // Update metrics tables
        updateMetrics(data);
        
        // Update charts if Chart.js is loaded
        if (chartJsLoaded) {
            try {
                // Ensure charts are initialized
                if (!chartsInitialized && typeof initCharts === 'function') {
                    chartsInitialized = initCharts();
                    if (chartsInitialized) {
                        console.log("Charts initialized on first data load");
                    }
                }
                
                // Add data to history and update charts
                addToHistory(data);
                updateCharts();
                console.log(`Charts updated with data points: ${dataPointsCollected}`);
            } catch (chartErr) {
                console.error("Error updating charts:", chartErr);
            }
        } else {
            console.warn("Chart.js not loaded, skipping chart updates");
        }
        
        // Update status
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
    const { requests: r, uploads: u, downloads: d, memory: m } = data;

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

    // Memory metrics
    if (m && m.available) {
        // Show memory data
        document.getElementById('memory_current').textContent = m.current_mb.toFixed(2) + ' MB';
        document.getElementById('memory_peak').textContent = m.peak_mb.toFixed(2) + ' MB';
        document.getElementById('memory_unavailable').style.display = 'none';
        
        // Show the individual metric lines
        const memoryInfo = document.querySelector('#memory_card .metric-info');
        const memoryDivs = memoryInfo.querySelectorAll('div:not(.memory-unavailable)');
        memoryDivs.forEach(div => div.style.display = 'block');
    } else {
        // Hide memory data and show unavailable message
        document.getElementById('memory_current').textContent = '-';
        document.getElementById('memory_peak').textContent = '-';
        document.getElementById('memory_unavailable').style.display = 'block';
        
        // Hide the individual metric lines
        const memoryInfo = document.querySelector('#memory_card .metric-info');
        const memoryDivs = memoryInfo.querySelectorAll('div:not(.memory-unavailable)');
        memoryDivs.forEach(div => div.style.display = 'none');
    }
}

function clearMetrics() {
    // Clear all metric values on error
    const metricElements = [
        'req_total', 'req_success', 'req_errors', 'req_success_rate',
        'bytes_served', 'mb_served',
        'up_total', 'up_success', 'up_failed', 'files_uploaded',
        'up_bytes', 'up_mb', 'avg_file_size', 'largest_upload',
        'concurrent_uploads', 'avg_processing', 'upload_success_rate',
        'uptime_secs', 'uptime_pretty',
        'memory_current', 'memory_peak'
    ];

    metricElements.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = '-';
    });
    
    // Hide memory unavailable message on error
    document.getElementById('memory_unavailable').style.display = 'none';
    // Show the individual metric lines
    const memoryInfo = document.querySelector('#memory_card .metric-info');
    const memoryDivs = memoryInfo.querySelectorAll('div:not(.memory-unavailable)');
    memoryDivs.forEach(div => div.style.display = 'block');
}

function updateTimestamp() {
    const now = new Date();
    const timeString = now.toLocaleTimeString();
    document.getElementById('last_updated').textContent = timeString;
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
        const uploadsElement = document.getElementById('uploadsChart');
        
        if (!requestsElement || !memoryElement || !uploadsElement) {
            console.error("Chart elements not found in DOM:", { 
                requests: !!requestsElement, 
                memory: !!memoryElement, 
                uploads: !!uploadsElement 
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
        
        // Uploads Chart
        const uploadsCtx = uploadsElement.getContext('2d');
        uploadsChart = new Chart(uploadsCtx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [
                    {
                        label: 'Total Uploads',
                        data: [],
                        borderColor: '#9CC8FF',
                        backgroundColor: 'rgba(156, 200, 255, 0.1)',
                        fill: false
                    },
                    {
                        label: 'Successful',
                        data: [],
                        borderColor: '#4ADE80',
                        backgroundColor: 'rgba(74, 222, 128, 0.1)',
                        fill: false
                    },
                    {
                        label: 'Failed',
                        data: [],
                        borderColor: '#F87171',
                        backgroundColor: 'rgba(248, 113, 113, 0.1)',
                        fill: false
                    },
                    {
                        label: 'Upload Size (MB)',
                        data: [],
                        borderColor: '#A78BFA',
                        backgroundColor: 'rgba(167, 139, 250, 0.1)',
                        fill: false,
                        yAxisID: 'y1'
                    }
                ]
            },
            options: {
                ...commonOptions,
                scales: {
                    ...commonOptions.scales,
                    y1: {
                        position: 'right',
                        beginAtZero: true,
                        grid: {
                            drawOnChartArea: false
                        },
                        ticks: {
                            color: '#A78BFA',
                            font: {
                                family: "'Inter', sans-serif",
                                size: 10
                            }
                        }
                    }
                }
            }
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
    if (!chartsContainer) {
        console.error("Charts container not found!");
        // Try to find chart elements directly
        const requestsChart = document.getElementById('requestsChart');
        const memoryChart = document.getElementById('memoryChart');
        const uploadsChart = document.getElementById('uploadsChart');
        console.log("Direct chart element checks:", {
            requestsChart: !!requestsChart,
            memoryChart: !!memoryChart,
            uploadsChart: !!uploadsChart
        });
    } else {
        console.log("Charts container found with children:", chartsContainer.children.length);
        // Log all chart canvas elements
        const canvases = chartsContainer.querySelectorAll('canvas');
        console.log("Canvas elements found:", canvases.length);
        canvases.forEach((canvas, i) => {
            console.log(`Canvas #${i} id:`, canvas.id);
        });
        
        // Check fallback elements
        const fallbacks = chartsContainer.querySelectorAll('.chart-fallback');
        console.log("Fallback elements found:", fallbacks.length);
    }
    
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

    // Auto-refresh every 2 seconds
    setInterval(loadMetrics, 2000);

    // Add manual refresh capability (optional)
    document.addEventListener('keydown', (e) => {
        if (e.key === 'r' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            loadMetrics();
        }
    });
    
    console.log("Monitor initialization complete");
}

// Start when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
