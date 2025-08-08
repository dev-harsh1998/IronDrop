# IronDrop Template System v2.0

## Overview

The new IronDrop template system provides a unified, maintainable, and consistent UI across all pages while maintaining the zero-dependency philosophy. The system uses a common base design with page-specific extensions.

## Architecture

### 1. Common Base System (`/templates/common/`)

#### `base.css` - Core Design System
- **CSS Variables**: Centralized design tokens for colors, typography, spacing, shadows
- **Base Components**: Buttons, cards, forms, tables, layout utilities
- **Typography**: Fira Code for logo/monospace, Inter for body text
- **Responsive Design**: Mobile-first approach with consistent breakpoints
- **Animations**: Fade-in, pulse, ripple effects

#### `base.html` - Template Structure (Reference)
- Common HTML structure with placeholders for customization
- Consistent header with IronDrop logo using Fira Code
- Footer with server information
- Placeholder sections for page-specific content

### 2. Page-Specific Extensions

#### Directory Listing (`/templates/directory/`)
- **`directory.css`**: File listing styles, table enhancements
- **`index_new.html`**: Updated template using base system
- **`script.js`**: Enhanced with loading animations and interactions

#### Upload Interface (`/templates/upload/`)
- **`upload.css`**: Drop zone, progress bars, queue management
- **`page_new.html`**: Drag & drop with touch support
- **`script.js`**: Mobile touch events, file validation

#### Error Pages (`/templates/error/`)
- **`error.css`**: Centered layout, error animations
- **`page_new.html`**: Professional error display
- **`script.js`**: Keyboard shortcuts, auto-redirect

## Design Principles

### 1. Unified Branding
- **Logo**: "IronDrop" in Fira Code font across all pages
- **Header**: Consistent navigation with logo on left, actions on right
- **Footer**: Unified server information display

### 2. Professional Dark Theme
- **Colors**: Blackish-grey palette with white accents
- **Glass Effects**: Backdrop blur with subtle borders
- **Shadows**: Layered shadows for depth
- **Gradients**: Subtle background gradients

### 3. Mobile-First Design
- **Touch Support**: Enhanced touch events for mobile upload
- **Responsive Layout**: Fluid design adapting to all screen sizes
- **Accessibility**: Proper contrast ratios and touch targets

### 4. Zero Dependencies
- **No External Libraries**: Pure CSS and vanilla JavaScript
- **Web Fonts**: Only Google Fonts for typography (Fira Code + Inter)
- **Custom Components**: All UI components built from scratch

## CSS Variable System

```css
:root {
    /* Colors */
    --bg-primary: #0a0a0a;           /* Deep black */
    --bg-secondary: #1a1a1a;         /* Dark grey */
    --bg-tertiary: #2a2a2a;          /* Medium grey */
    --text-primary: #e5e5e5;         /* Light grey */
    --text-accent: #ffffff;          /* Pure white accent */
    
    /* Typography */
    --font-family: 'Fira Code', monospace;  /* Logo & code */
    --font-body: 'Inter', sans-serif;       /* Body text */
    
    /* Spacing */
    --space-xs: 0.25rem;
    --space-sm: 0.5rem;
    --space-md: 1rem;
    --space-lg: 1.5rem;
    --space-xl: 2rem;
    --space-2xl: 3rem;
    
    /* Effects */
    --shadow: 0 25px 35px -5px rgba(0, 0, 0, 0.8);
    --gradient-primary: linear-gradient(135deg, #2a2a2a 0%, #1a1a1a 100%);
}
```

## Component Library

### Buttons
- `.btn` - Base button class
- `.btn-primary` - Accent gradient button
- `.btn-secondary` - Glass effect button
- `.btn-ghost` - Minimal border button

### Cards
- `.card` - Glass container with blur effect
- `.card-header` - Header section
- `.card-content` - Main content area
- `.card-footer` - Footer section

### Tables
- `.table-container` - Wrapper with glass effect
- `.table` - Professional table styling
- Row hover effects and striping

### Forms
- `.form-group` - Form field wrapper
- `.form-label` - Consistent label styling
- `.form-input` - Input field with focus states

## Migration Guide

### 1. File Structure Changes
```
templates/
├── common/
│   ├── base.css         # New: Core design system
│   └── base.html        # New: Reference template
├── directory/
│   ├── directory.css    # New: Directory-specific styles
│   ├── index_new.html   # New: Updated template
│   └── index.html       # Old: To be replaced
├── upload/
│   ├── upload.css       # New: Upload-specific styles
│   ├── page_new.html    # New: Updated template
│   └── page.html        # Old: To be replaced
└── error/
    ├── error.css        # New: Error-specific styles
    ├── page_new.html    # New: Updated template
    └── page.html        # Old: To be replaced
```

### 2. Implementation Steps

1. **Add Common Base**:
   - Deploy `common/base.css` to static assets
   - Ensure Rust server serves `/_static/common/base.css`

2. **Update Page Templates**:
   - Replace existing HTML files with new versions
   - Update CSS file references in template engine

3. **Deploy Page-Specific Styles**:
   - Deploy new CSS files to respective directories
   - Test responsive behavior and animations

### 3. Backward Compatibility
- Old templates remain functional during transition
- New system can be deployed incrementally
- No breaking changes to existing URLs or functionality

## Benefits

### 1. Maintainability
- **Single Source of Truth**: All design tokens in base.css
- **Consistent Updates**: Change base variables to update all pages
- **Modular Structure**: Page-specific styles extend base system

### 2. Performance
- **Optimized CSS**: Reduced duplication, smaller file sizes
- **Efficient Loading**: Shared base styles cached across pages
- **Modern Techniques**: CSS variables, backdrop-filter effects

### 3. User Experience
- **Professional Appearance**: Consistent branding and styling
- **Mobile Optimized**: Touch-friendly interactions
- **Accessibility**: Proper contrast and keyboard navigation

### 4. Developer Experience
- **Clear Structure**: Logical separation of concerns
- **Easy Customization**: CSS variables for quick theming
- **Comprehensive Documentation**: Clear usage guidelines

## Future Enhancements

1. **Theme Support**: Light mode variants using CSS variables
2. **Component Extensions**: Additional UI components as needed
3. **Animation Library**: More sophisticated transitions
4. **Print Styles**: Optimized layouts for printing
5. **High Contrast Mode**: Enhanced accessibility options

## Testing Checklist

- [ ] All pages load with consistent header/footer
- [ ] Logo displays correctly in Fira Code font
- [ ] Responsive design works on mobile devices
- [ ] Touch interactions function properly
- [ ] Dark theme maintains contrast ratios
- [ ] File upload drag & drop operates smoothly
- [ ] Error pages display with proper styling
- [ ] Navigation between pages maintains consistency
