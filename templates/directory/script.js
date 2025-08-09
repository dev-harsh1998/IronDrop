// Dark Mode Only Directory Listing Enhancements with Fast Search
document.addEventListener('DOMContentLoaded', function() {
    // Apply loading animation with staggered effect
    const container = document.querySelector('.container');
    const header = document.querySelector('.directory-header');
    const listing = document.querySelector('.listing');
    const footer = document.querySelector('.server-footer');
    
    container.classList.add('loading');

    // Staggered animation for different sections
    setTimeout(() => {
        if (header) header.style.opacity = '1';
    }, 100);

    setTimeout(() => {
        if (listing) listing.style.opacity = '1';
    }, 200);

    setTimeout(() => {
        if (footer) footer.style.opacity = '1';
    }, 300);

    // Initial opacity for sections
    if (header) header.style.opacity = '0';
    if (listing) listing.style.opacity = '0';
    if (footer) footer.style.opacity = '0';
    
    const rows = document.querySelectorAll('tbody tr');
    const totalFiles = rows.length;
    
    // Global variables for search functionality
    let dropdown = null;
    let selectedDropdownIndex = -1;
    let isSearchActive = false;
    // Smooth scrolling for large directories
    if (totalFiles > 50) {
        document.body.style.scrollBehavior = 'smooth';
    }

    // Performance optimization for large directories
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
            }
        });
    }, {
        threshold: 0.1,
        rootMargin: '50px'
    });

    // Apply intersection observer for very large directories
    if (rows.length > 100) {
        rows.forEach(row => {
            row.style.opacity = '0.7';
            observer.observe(row);
        });
    }

    // Keyboard navigation enhancements
    document.addEventListener('keydown', function(e) {
        // Check if search is active (search input is focused or dropdown is visible)
        const searchInput = document.getElementById('search');
        isSearchActive = document.activeElement === searchInput || dropdown;
        
        // Skip handling if user is typing in a form element (except search)
        const activeElement = document.activeElement;
        const isInputActive = activeElement && 
            (activeElement.tagName === 'INPUT' || activeElement.tagName === 'TEXTAREA') &&
            activeElement.id !== 'search';
        
        if (isInputActive) return;
        
        // If search is active, handle search-specific navigation
        if (isSearchActive) {
            handleSearchKeydown(e);
            return;
        }
        
        // Regular file navigation when search is not active
        handleFileNavigation(e);
    });
    
    function handleSearchKeydown(e) {
        const searchInput = document.getElementById('search');
        
        // Ctrl/Cmd + F to focus search
        if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
            e.preventDefault();
            searchInput.focus();
            searchInput.select();
            // Announce to screen readers
            announceToScreenReader('Search focused');
        }
        
        // Escape to clear search
        if (e.key === 'Escape' && document.activeElement === searchInput) {
            e.preventDefault();
            if (dropdown) {
                hideDropdown();
                announceToScreenReader('Search suggestions closed');
            } else {
                const hadValue = searchInput.value.length > 0;
                searchInput.value = '';
                // Clear timeout to prevent any pending searches
                clearTimeout(searchTimeout);
                showAllRows();
                searchInput.blur();
                if (hadValue) {
                    announceToScreenReader('Search cleared, showing all items');
                }
            }
        }
        
        // Arrow keys to navigate dropdown
        if (dropdown && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
            e.preventDefault();
            navigateDropdown(e.key === 'ArrowDown' ? 1 : -1);
        }
        
        // Enter to select dropdown item
        if (dropdown && e.key === 'Enter' && document.activeElement === searchInput) {
            e.preventDefault();
            const selected = dropdown.querySelector('.dropdown-item.selected');
            if (selected) {
                selected.click();
            }
        }
    }
    
    function handleFileNavigation(e) {
        // Arrow key navigation for files
        if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
            e.preventDefault();
            navigateFiles(e.key === 'ArrowDown' ? 1 : -1);
        }

        // Enter to follow link
        if (e.key === 'Enter') {
            const selected = document.querySelector('.file-link.selected');
            if (selected) {
                window.location.href = selected.href;
            }
        }

        // Home/End navigation
        if (e.key === 'Home') {
            e.preventDefault();
            selectFile(0);
        }
        if (e.key === 'End') {
            e.preventDefault();
            selectFile(rows.length - 1);
        }
    }
    let selectedIndex = -1;

    function navigateFiles(direction) {
        const links = document.querySelectorAll('.file-link');
        if (links.length === 0) return;

        // Remove current selection
        links.forEach(link => link.classList.remove('selected'));

        // Update index
        selectedIndex += direction;
        if (selectedIndex < 0) selectedIndex = links.length - 1;
        if (selectedIndex >= links.length) selectedIndex = 0;

        // Add selection to new file
        selectFile(selectedIndex);
    }

    function selectFile(index) {
        const links = document.querySelectorAll('.file-link');
        if (index < 0 || index >= links.length) return;

        // Remove all selections
        links.forEach(link => link.classList.remove('selected'));

        // Add selection
        selectedIndex = index;
        const selected = links[selectedIndex];
        selected.classList.add('selected');

        // Scroll into view
        selected.scrollIntoView({
            behavior: 'smooth',
            block: 'center'
        });
    }
    
    // Add selected file styling and focus indicators
    const style = document.createElement('style');
    style.textContent = `
        .file-link.selected,
        .file-link:focus {
            background: rgba(96, 165, 250, 0.2);
            border-radius: var(--radius-medium);
            padding: 0.5rem;
            margin: -0.5rem;
            outline: 2px solid rgba(96, 165, 250, 0.5);
            outline-offset: 2px;
        }
        
        .search-input:focus {
            outline: 3px solid rgba(96, 165, 250, 0.5);
            outline-offset: 2px;
            transition: all var(--transition-normal);
        }
        
        /* High contrast mode support */
        @media (prefers-contrast: high) {
            .file-link:focus,
            .file-link.selected {
                outline: 3px solid;
                outline-color: Highlight;
            }
            
            .search-input:focus {
                outline: 3px solid;
                outline-color: Highlight;
            }
        }
    `;
    document.head.appendChild(style);
    
    // Screen reader announcement function
    function announceToScreenReader(message) {
        const announcement = document.createElement('div');
        announcement.setAttribute('aria-live', 'polite');
        announcement.setAttribute('aria-atomic', 'true');
        announcement.className = 'sr-only';
        announcement.textContent = message;
        document.body.appendChild(announcement);
        
        // Remove after announcement
        setTimeout(() => {
            document.body.removeChild(announcement);
        }, 1000);
    }
    // File type detection for better visual indicators
    document.querySelectorAll('.file-link').forEach(link => {
        const fileName = link.querySelector('.name').textContent;
        const extension = fileName.split('.').pop().toLowerCase();

        const fileType = link.querySelector('.file-type');
        if (fileType && !fileType.classList.contains('directory')) {
            // Add specific colors for different file types
            switch (extension) {
                case 'txt':
                case 'md':
                case 'readme':
                    fileType.style.background = 'linear-gradient(135deg, #ffffff, #cccccc)';
                    break;
                case 'js':
                case 'ts':
                case 'json':
                    fileType.style.background = 'linear-gradient(135deg, #cccccc, #999999)';
                    break;
                case 'html':
                case 'css':
                case 'scss':
                    fileType.style.background = 'linear-gradient(135deg, #999999, #777777)';
                    break;
                case 'png':
                case 'jpg':
                case 'jpeg':
                case 'gif':
                case 'svg':
                    fileType.style.background = 'linear-gradient(135deg, #777777, #555555)';
                    break;
                case 'zip':
                case 'tar':
                case 'gz':
                case 'rar':
                    fileType.style.background = 'linear-gradient(135deg, #555555, #333333)';
                    break;
                default:
                    // Apply default blackish grey gradient for unknown file types
                    fileType.style.background = 'linear-gradient(135deg, #888888, #555555)';
                    break;
            }
        }
    });
    
    // Initialize search functionality for directories with files
    if (totalFiles > 0) {
        // Ensure all rows start in visible state to prevent layout shifts
        rows.forEach(row => {
            row.classList.add('visible');
        });
        
        initializeSearch(rows, totalFiles);
    }
    
    // Initialize search functionality
    function initializeSearch(rows, totalFiles) {
        const searchInput = document.getElementById('search');
        const searchStatus = document.getElementById('search-status');
        
        if (!searchInput || !searchStatus) return;
        
        // Update placeholder with item count (files and directories)
        const fileCount = Array.from(rows).filter(row => {
            const fileTypeEl = row.querySelector('.file-type');
            return fileTypeEl && !fileTypeEl.classList.contains('directory');
        }).length;
        const dirCount = totalFiles - fileCount;
        
        if (dirCount > 0) {
            searchInput.placeholder = `Search ${fileCount} files, ${dirCount} directories...`;
        } else {
            searchInput.placeholder = `Search ${totalFiles} files...`;
        }
        searchStatus.textContent = `${totalFiles} items`;
        
        // Build search index
        const searchIndex = buildSearchIndex(rows);
        
        // Search engine
        let searchTimeout;
        
        // Add keyup handler to catch delete/backspace events that clear the input
        searchInput.addEventListener('keyup', function(e) {
            if (e.key === 'Backspace' || e.key === 'Delete') {
                const query = e.target.value.trim();
                if (!query) {
                    hideDropdown();
                    showAllRows();
                    searchStatus.classList.remove('loading', 'has-results');
                    resetDropdownSelection();
                }
            }
        });
        
        // Search input handler with debouncing
        searchInput.addEventListener('input', function(e) {
            clearTimeout(searchTimeout);
            const query = e.target.value.trim();
            
            // Immediately clear any existing dropdown and reset state
            hideDropdown();
            
            if (!query) {
                // Ensure complete cleanup when search is cleared
                showAllRows();
                // Force cleanup of any remaining state
                searchStatus.classList.remove('loading', 'has-results');
                // Reset any selected dropdown state
                resetDropdownSelection();
                return;
            }
            
            // Show loading state immediately for better UX
            searchStatus.classList.add('loading');
            
            // Immediate feedback for very short queries to prevent UI jumping
            if (query.length === 1) {
                // Show quick preview for single character
                const quickResults = searchIndex.filter(item => 
                    item.name.startsWith(query.toLowerCase())
                );
                searchStatus.textContent = `${quickResults.length} matches`;
            } else {
                searchStatus.textContent = 'Searching...';
            }
            
            // Debounce search for performance with shorter delay for better responsiveness
            searchTimeout = setTimeout(() => {
                // Remove loading state
                searchStatus.classList.remove('loading');
                
                // Perform local search first for current directory
                performSearch(query);
                
                // Then perform API search for subdirectories (if query is long enough)
                if (query.length >= 2) {
                    performApiSearch(query);
                }
            }, 100); // Reduced from 150ms to 100ms for better responsiveness
        });
        
        // Note: Keyboard shortcuts are now handled in the main event listener above
        
        function buildSearchIndex(rows) {
            const index = [];
            console.log(`Building search index for ${rows.length} rows`);
            
            rows.forEach((row, i) => {
                const nameEl = row.querySelector('.name');
                const sizeEl = row.querySelector('.size');
                const fileTypeEl = row.querySelector('.file-type');
                
                if (nameEl && nameEl.textContent) {
                    const originalName = nameEl.textContent.trim();
                    const name = originalName.toLowerCase();
                    const isDirectory = fileTypeEl && fileTypeEl.classList.contains('directory');
                    
                    const fileInfo = {
                        idx: i,
                        row: row,
                        name: name,
                        nameEl: nameEl,
                        originalName: originalName,
                        size: sizeEl ? sizeEl.textContent : '',
                        isDirectory: isDirectory,
                        type: isDirectory ? 'directory' : 'file',
                        tokens: name.split(/[\s\-_.]+/).filter(t => t.length > 0)
                    };
                    
                    index.push(fileInfo);
                    console.log(`Indexed: "${originalName}" (${fileInfo.type}) -> tokens: ${fileInfo.tokens.join(', ')}`);
                }
            });
            
            console.log(`Search index built with ${index.length} items (files and directories)`);
            return index;
        }
        
        function performSearch(query) {
            const start = performance.now();
            const queryLower = query.toLowerCase();
            const queryParts = queryLower.split(/\s+/).filter(p => p.length > 0);
            const results = [];
            
            // Debug logging
            console.log(`Searching for: "${query}" in ${searchIndex.length} items (files and directories)`);
            
            // Enhanced search that works for both files and directories
            searchIndex.forEach(item => {
                let matches = false;
                let matchScore = 0;
                
                // Check if all query parts are found in the name (works for both files and directories)
                if (queryParts.every(part => item.name.includes(part))) {
                    matches = true;
                    matchScore += 3; // Exact substring match gets high score
                } 
                // Fuzzy matching for typos (works for both files and directories)
                else if (fuzzyMatch(item.name, queryLower)) {
                    matches = true;
                    matchScore += 2;
                }
                // Token-based matching for names with separators
                else if (tokenMatch(item.tokens, queryParts)) {
                    matches = true;
                    matchScore += 1;
                }
                
                if (matches) {
                    item.matchScore = matchScore;
                    results.push(item);
                }
            });
            
            // Log search results breakdown
            const fileResults = results.filter(r => r.type === 'file').length;
            const dirResults = results.filter(r => r.type === 'directory').length;
            console.log(`Search results: ${fileResults} files, ${dirResults} directories (${results.length} total)`);
            
            // Limit results for very large directories
            if (results.length > 100) {
                results.splice(100);
            }
            
            // Enhanced sorting by relevance and type
            results.sort((a, b) => {
                // First sort by match score
                if (a.matchScore !== b.matchScore) return b.matchScore - a.matchScore;
                
                // Then directories before files for same match score
                if (a.isDirectory !== b.isDirectory) return b.isDirectory - a.isDirectory;
                
                // Then exact matches
                const aExact = a.name.includes(queryLower);
                const bExact = b.name.includes(queryLower);
                if (aExact !== bExact) return bExact - aExact;
                
                // Then prefix matches
                const aPrefix = a.name.startsWith(queryLower);
                const bPrefix = b.name.startsWith(queryLower);
                if (aPrefix !== bPrefix) return bPrefix - aPrefix;
                
                // Finally shorter names
                return a.name.length - b.name.length;
            });
            
            updateDOM(results, queryLower);
            
            const elapsed = performance.now() - start;
            if (elapsed > 10) {
                console.warn(`Search took ${elapsed.toFixed(2)}ms for ${searchIndex.length} items`);
            }
        }
        
        
        function fuzzyMatch(filename, query) {
            let qIdx = 0;
            for (let i = 0; i < filename.length && qIdx < query.length; i++) {
                if (filename[i] === query[qIdx]) {
                    qIdx++;
                }
            }
            return qIdx === query.length;
        }
        
        function tokenMatch(tokens, queryParts) {
            return queryParts.every(part => 
                tokens.some(token => token.startsWith(part))
            );
        }
        
        function updateDOM(results, query) {
            // Immediately update search status to prevent UI jumping
            const count = results.length;
            const total = searchIndex.length;
            const fileResults = results.filter(r => r.type === 'file').length;
            const dirResults = results.filter(r => r.type === 'directory').length;
            
            let statusText;
            if (count === total) {
                statusText = `${total} items`;
            } else if (dirResults > 0 && fileResults > 0) {
                statusText = `${fileResults}f, ${dirResults}d`; // Shorter text to prevent overflow
            } else if (dirResults > 0) {
                statusText = `${dirResults} dirs`;
            } else {
                statusText = `${fileResults} files`;
            }
            
            searchStatus.textContent = statusText;
            searchStatus.classList.toggle('has-results', count > 0 && count < total);
            
            // Use requestAnimationFrame for smooth DOM updates
            requestAnimationFrame(() => {
                // Batch DOM operations for better performance
                const toShow = [];
                const toHide = [];
                
                searchIndex.forEach(item => {
                    if (results.includes(item)) {
                        toShow.push(item);
                    } else {
                        toHide.push(item);
                    }
                });
                
                // Apply changes simultaneously to prevent visual jumping
                toHide.forEach(item => {
                    item.row.classList.add('hidden');
                    item.row.classList.remove('visible', 'search-match');
                    clearHighlight(item.nameEl);
                });
                
                toShow.forEach(item => {
                    item.row.classList.remove('hidden');
                    item.row.classList.add('visible');
                    // Add subtle highlight animation for fewer results only
                    if (results.length < 15) {
                        // Use a shorter, less intrusive animation
                        setTimeout(() => {
                            item.row.classList.add('search-match');
                            setTimeout(() => item.row.classList.remove('search-match'), 300);
                        }, 10);
                    }
                    highlightMatch(item.nameEl, item.originalName, query);
                });
            });
        }
        
        function highlightMatch(element, originalText, query) {
            try {
                const lowerText = originalText.toLowerCase();
                const lowerQuery = query.toLowerCase();
                const idx = lowerText.indexOf(lowerQuery);
                
                if (idx !== -1) {
                    // Use safe HTML creation
                    const beforeMatch = originalText.slice(0, idx);
                    const matchText = originalText.slice(idx, idx + query.length);
                    const afterMatch = originalText.slice(idx + query.length);
                    
                    // Clear existing content
                    element.textContent = '';
                    
                    // Add text nodes and highlighted match
                    if (beforeMatch) element.appendChild(document.createTextNode(beforeMatch));
                    
                    const mark = document.createElement('mark');
                    mark.textContent = matchText;
                    element.appendChild(mark);
                    
                    if (afterMatch) element.appendChild(document.createTextNode(afterMatch));
                } else {
                    element.textContent = originalText;
                }
            } catch (error) {
                console.warn('Error highlighting match:', error);
                element.textContent = originalText;
            }
        }
        
        function clearHighlight(element) {
            const text = element.textContent;
            element.textContent = text; // This removes any HTML tags
        }
        
        function showAllRows() {
            // Immediately update status to prevent UI jumping
            searchStatus.textContent = `${totalFiles} items`;
            searchStatus.classList.remove('has-results', 'loading');
            
            // Ensure dropdown is completely hidden
            hideDropdown();
            
            // Then update DOM with smooth transitions
            requestAnimationFrame(() => {
                searchIndex.forEach(item => {
                    item.row.classList.remove('hidden', 'search-match');
                    item.row.classList.add('visible');
                    clearHighlight(item.nameEl);
                });
            });
        }
        
        // API search for subdirectories
        async function performApiSearch(query) {
            try {
                // Verify the query is still current before making API call
                const currentQuery = searchInput.value.trim();
                if (currentQuery !== query || !currentQuery) {
                    console.log('Query changed during API search, aborting');
                    return;
                }
                
                const currentPath = window.location.pathname;
                const response = await fetch(`/_irondrop/search?q=${encodeURIComponent(query)}&path=${encodeURIComponent(currentPath)}`);
                
                if (!response.ok) {
                    console.warn('API search failed:', response.status);
                    return;
                }
                
                const results = await response.json();
                console.log(`API search found ${results.length} results`);
                
                // Double-check query is still current after API response
                const finalQuery = searchInput.value.trim();
                if (finalQuery !== query || !finalQuery) {
                    console.log('Query changed after API response, ignoring results');
                    return;
                }
                
                // Show dropdown with results
                if (results.length > 0) {
                    showDropdown(results, query);
                    announceToScreenReader(`Found ${results.length} additional results in subdirectories`);
                }
                
            } catch (error) {
                console.warn('API search error:', error);
            }
        }
        
        // Note: dropdown and selectedDropdownIndex are now global variables
        
        function showDropdown(results, query) {
            // Remove existing dropdown with proper cleanup
            if (dropdown) {
                hideDropdown();
                // Wait for cleanup to complete before creating new dropdown
                setTimeout(() => createDropdown(results, query), 160);
            } else {
                createDropdown(results, query);
            }
        }
        
        function createDropdown(results, query) {
            if (results.length === 0) return;
            
            // Verify query is still current
            const currentQuery = searchInput.value.trim();
            if (currentQuery !== query || !currentQuery) {
                return;
            }
            
            // Create dropdown
            dropdown = document.createElement('div');
            dropdown.className = 'search-dropdown';
            dropdown.style.opacity = '0';
            dropdown.style.transform = 'translateY(-10px)';
            dropdown.innerHTML = `
                <div class="dropdown-header">
                    <span>Files in subdirectories (${results.length})</span>
                </div>
                <div class="dropdown-results"></div>
            `;
            
            const dropdownResults = dropdown.querySelector('.dropdown-results');
            
            // Add results
            results.forEach(result => {
                const item = document.createElement('div');
                item.className = 'dropdown-item';
                
                const icon = result.type === 'directory' ? '📁' : '📄';
                const highlightedName = highlightText(result.name, query);
                
                item.innerHTML = `
                    <span class="dropdown-icon">${icon}</span>
                    <div class="dropdown-info">
                        <div class="dropdown-name">${highlightedName}</div>
                        <div class="dropdown-path">${result.path}</div>
                    </div>
                    <div class="dropdown-size">${result.size}</div>
                `;
                
                item.addEventListener('click', () => {
                    window.location.href = result.path;
                });
                
                dropdownResults.appendChild(item);
            });
            
            // Position dropdown
            const searchContainer = document.querySelector('.search-container');
            searchContainer.appendChild(dropdown);
            
            // Animate in
            requestAnimationFrame(() => {
                dropdown.style.opacity = '1';
                dropdown.style.transform = 'translateY(0)';
                dropdown.style.pointerEvents = 'auto';
            });
        }
        
        function hideDropdown() {
            if (dropdown) {
                // Add fade out animation before removal for smoother transition
                dropdown.style.opacity = '0';
                dropdown.style.transform = 'translateY(-10px)';
                dropdown.style.pointerEvents = 'none';
                
                // Remove after animation completes
                setTimeout(() => {
                    if (dropdown && dropdown.parentNode) {
                        dropdown.remove();
                    }
                    dropdown = null;
                    resetDropdownSelection();
                }, 150);
            }
        }
        
        function highlightText(text, query) {
            const index = text.toLowerCase().indexOf(query.toLowerCase());
            if (index === -1) return text;
            
            return text.slice(0, index) + 
                   '<mark>' + text.slice(index, index + query.length) + '</mark>' +
                   text.slice(index + query.length);
        }
        
        // Dropdown navigation functions
        
        function navigateDropdown(direction) {
            if (!dropdown) return;
            
            const items = dropdown.querySelectorAll('.dropdown-item');
            if (items.length === 0) return;
            
            // Remove current selection
            items.forEach(item => item.classList.remove('selected'));
            
            // Update index
            selectedDropdownIndex += direction;
            if (selectedDropdownIndex < 0) selectedDropdownIndex = items.length - 1;
            if (selectedDropdownIndex >= items.length) selectedDropdownIndex = 0;
            
            // Add selection to new item
            const selectedItem = items[selectedDropdownIndex];
            selectedItem.classList.add('selected');
            selectedItem.scrollIntoView({ block: 'nearest' });
        }
        
        // Reset dropdown selection when dropdown is hidden
        function resetDropdownSelection() {
            selectedDropdownIndex = -1;
        }
    }
});