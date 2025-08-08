// Dark Mode Only Directory Listing Enhancements
document.addEventListener('DOMContentLoaded', function () {
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

    // Smooth scrolling for large directories
    if (document.querySelectorAll('tbody tr').length > 50) {
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
    const rows = document.querySelectorAll('tbody tr');
    if (rows.length > 100) {
        rows.forEach(row => {
            row.style.opacity = '0.7';
            observer.observe(row);
        });
    }

    // Keyboard navigation enhancements
    document.addEventListener('keydown', function (e) {
        // Arrow key navigation
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
    });

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

    // Add selected file styling
    const style = document.createElement('style');
    style.textContent = `
        .file-link.selected {
            background: rgba(96, 165, 250, 0.2);
            border-radius: 8px;
            padding: 0.5rem;
            margin: -0.5rem;
        }
    `;
    document.head.appendChild(style);

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
});