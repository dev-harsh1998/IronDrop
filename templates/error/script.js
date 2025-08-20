// SPDX-License-Identifier: MIT
// Dark Mode Only Error Page Enhancements
document.addEventListener('DOMContentLoaded', function() {
    // Keyboard shortcuts
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape' || e.key === 'h' || e.key === 'Home') {
            window.location.href = '/';
        }
        
        // Backspace to go back
        if (e.key === 'Backspace') {
            e.preventDefault();
            window.history.back();
        }
    });
    
    // Auto-redirect after timeout (optional)
    const urlParams = new URLSearchParams(window.location.search);
    const autoRedirect = urlParams.get('redirect');
    
    if (autoRedirect && autoRedirect !== 'false') {
        setTimeout(() => {
            window.location.href = '/';
        }, 5000); // 5 second auto-redirect
        
        // Show countdown
        let countdown = 5;
        const countdownElement = document.createElement('div');
        countdownElement.style.cssText = `
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            background: var(--bg-glass);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 1rem;
            color: var(--text-secondary);
            font-size: 0.875rem;
            backdrop-filter: blur(10px);
        `;
        countdownElement.textContent = `Redirecting in ${countdown}s`;
        document.body.appendChild(countdownElement);
        
        const interval = setInterval(() => {
            countdown--;
            countdownElement.textContent = `Redirecting in ${countdown}s`;
            if (countdown <= 0) {
                clearInterval(interval);
                countdownElement.remove();
            }
        }, 1000);
    }
    
    // Enhanced error reporting (optional)
    const errorCode = document.querySelector('.error-code').textContent;
    const errorData = {
        code: errorCode,
        path: window.location.pathname,
        timestamp: new Date().toISOString(),
        userAgent: navigator.userAgent
    };
    
    console.log('Error Details:', errorData);
    
    // Add ripple effect to back button
    const backLink = document.querySelector('.back-link');
    if (backLink) {
        backLink.addEventListener('click', function(e) {
            const ripple = document.createElement('span');
            const rect = this.getBoundingClientRect();
            const size = Math.max(rect.width, rect.height);
            const x = e.clientX - rect.left - size / 2;
            const y = e.clientY - rect.top - size / 2;
            
            ripple.style.cssText = `
                position: absolute;
                width: ${size}px;
                height: ${size}px;
                left: ${x}px;
                top: ${y}px;
                background: rgba(248, 113, 113, 0.3);
                border-radius: 50%;
                transform: scale(0);
                animation: ripple 0.6s linear;
                pointer-events: none;
            `;
            
            this.style.position = 'relative';
            this.style.overflow = 'hidden';
            this.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
        });
        
        // Add ripple animation
        const style = document.createElement('style');
        style.textContent = `
            @keyframes ripple {
                to {
                    transform: scale(4);
                    opacity: 0;
                }
            }
        `;
        document.head.appendChild(style);
    }
});