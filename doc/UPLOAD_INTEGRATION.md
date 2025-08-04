# IronDrop Upload UI Integration Guide v2.5

This document provides guidance on integrating the modern upload UI templates into IronDrop's file server functionality.

**Status**: Fully implemented and production-ready in IronDrop v2.5. The upload UI system is complete with professional design, comprehensive functionality, and extensive testing.

## Overview

The upload UI system consists of four main components:

1. **Upload Page Template** (`/templates/upload/page.html`) - Dedicated upload page with drag-and-drop interface
2. **Upload Styles** (`/templates/upload/styles.css`) - Modern CSS styling matching the existing theme  
3. **Upload Script** (`/templates/upload/script.js`) - JavaScript for drag-drop, AJAX uploads, and progress tracking
4. **Inline Upload Form** (`/templates/upload/form.html`) - Reusable component for directory listings

## Features Implemented

### 🎯 User Experience
- **Drag-and-Drop Interface**: Large drop zone with visual feedback
- **File Preview**: Shows file details, sizes, and progress
- **Progress Tracking**: Real-time upload progress with visual indicators
- **Multiple File Support**: Upload multiple files simultaneously
- **Responsive Design**: Works on desktop, tablet, and mobile devices

### 🎨 Visual Design
- **Dark Theme Integration**: Matches existing professional blackish-grey theme
- **Glass Morphism**: Uses backdrop filters and translucent elements
- **Smooth Animations**: CSS transitions and hover effects
- **Status Indicators**: Color-coded progress states (pending, uploading, completed, error)

### ⚙️ Technical Features
- **AJAX Upload**: Non-blocking file uploads with XMLHttpRequest
- **Concurrent Uploads**: Up to 3 simultaneous uploads
- **File Validation**: Client-side size and type validation
- **Error Handling**: Graceful error display and recovery
- **Auto-retry Logic**: Built-in retry mechanisms for failed uploads

## Integration Points

### Template Engine Updates

The `templates.rs` file has been updated with:

```rust
// New template constants
const UPLOAD_PAGE_HTML: &str = include_str!("../templates/upload/page.html");
const UPLOAD_STYLES_CSS: &str = include_str!("../templates/upload/styles.css");
const UPLOAD_SCRIPT_JS: &str = include_str!("../templates/upload/script.js");
const UPLOAD_FORM_HTML: &str = include_str!("../templates/upload/form.html");

// New template methods
pub fn render_upload_page(&self, path: &str) -> Result<String, AppError>
pub fn get_upload_form(&self) -> Result<String, AppError>
```

### Static Asset Serving

Upload assets are served via the existing static asset system:
- `/_static/upload/styles.css`
- `/_static/upload/script.js`

## Usage Examples

### 1. Dedicated Upload Page

To create a dedicated upload page (e.g., `/upload`):

```rust
// In your route handler
let upload_page = template_engine.render_upload_page(&current_path)?;
Response::builder()
    .status(200)
    .header("content-type", "text/html")
    .body(upload_page.into())
```

### 2. Inline Upload in Directory Listings

To add upload functionality to directory listings:

```rust
// Modify the directory template to include upload form
let mut variables = HashMap::new();
variables.insert("PATH".to_string(), path.to_string());
variables.insert("ENTRIES".to_string(), entries_html);

// Add upload form if uploads are enabled
if uploads_enabled {
    let upload_form = template_engine.get_upload_form()?;
    variables.insert("UPLOAD_FORM".to_string(), upload_form);
}
```

Then modify `templates/directory/index.html`:

```html
<div class="container">
    {{UPLOAD_FORM}}  <!-- Insert upload form here -->
    <div class="listing">
        <!-- existing table content -->
    </div>
</div>
```

### 3. Upload Endpoint Handling

The JavaScript expects a POST endpoint that accepts multipart form data:

```rust
// Example upload handler
if method == "POST" && query_params.contains("upload=true") {
    // Handle multipart/form-data uploads
    // Save files to the current directory
    // Return appropriate HTTP status codes
}
```

## File Structure

```
templates/
├── directory/
│   ├── index.html      # Existing directory listing
│   ├── styles.css      # Base styles
│   └── script.js       # Directory functionality
├── upload/
│   ├── page.html       # Dedicated upload page
│   ├── styles.css      # Upload-specific styles
│   ├── script.js       # Upload functionality  
│   └── form.html       # Reusable upload form component
└── error/
    ├── page.html       # Error pages
    ├── styles.css      # Error styles
    └── script.js       # Error functionality
```

## Styling Guidelines

### CSS Custom Properties
The upload UI uses the same CSS custom properties as the main theme:

```css
:root {
    --bg-primary: #0a0a0a;
    --bg-secondary: #1a1a1a;
    --bg-tertiary: #2a2a2a;
    --text-primary: #e5e5e5;
    --text-secondary: #b0b0b0;
    --text-accent: #ffffff;
    /* ... */
}
```

### Component Structure
Upload components follow the existing design patterns:
- Glass morphism effects with `backdrop-filter: blur(20px)`
- Rounded corners with `border-radius: 24px` for containers
- Consistent spacing using `2rem` padding
- Hover effects with `transform` and `box-shadow`

## JavaScript API

### UploadManager Class

```javascript
const uploadManager = new UploadManager();

// Key methods:
uploadManager.handleFiles(fileArray)        // Add files to queue
uploadManager.startUploads()               // Begin upload process
uploadManager.removeFile(fileId)           // Remove from queue
uploadManager.updateProgress(fileId, %)    // Update progress
```

### Events and Callbacks

The upload system emits various events for integration:
- File validation errors
- Upload progress updates
- Completion notifications
- Error handling

## Security Considerations

### File Validation
- **Client-side**: Size limits (100MB default), type checking
- **Server-side**: Additional validation should be implemented
- **Path sanitization**: Ensure uploaded files don't escape intended directory

### Error Handling
- Graceful degradation for unsupported browsers
- Clear error messages for validation failures
- Network error recovery with retry options

## Browser Compatibility

- **Modern browsers**: Full drag-and-drop support with progress tracking
- **Older browsers**: Fallback to standard file input with form submission
- **Mobile devices**: Touch-optimized interface with file picker integration

## Performance Considerations

- **Concurrent uploads**: Limited to 3 simultaneous transfers
- **Memory usage**: Files are streamed, not loaded entirely into memory
- **Progress tracking**: Efficient DOM updates with minimal reflows
- **Asset loading**: CSS and JS are embedded and compressed

## Customization

### Theming
Upload styles can be customized by modifying the CSS custom properties in `upload/styles.css`.

### File Type Icons
Add custom file type detection in the JavaScript:

```javascript
getFileTypeIcon(filename) {
    const extension = filename.split('.').pop().toLowerCase();
    // Return appropriate SVG icon based on extension
}
```

### Upload Limits
Modify validation in both client and server code:

```javascript
// Client-side validation
const maxSize = 100 * 1024 * 1024; // 100MB
const allowedTypes = ['*']; // All types allowed
```

## Current Implementation Status

### ✅ **Fully Implemented Features (v2.5)**

- **Complete Upload System**: Production-ready file upload handling
- **Professional UI**: Modern blackish-grey theme with glassmorphism effects
- **Template Integration**: All templates embedded and served via `/_static/` routes
- **Security Integration**: Upload validation respects CLI security configurations
- **Multi-file Support**: Concurrent upload handling with progress tracking
- **Error Handling**: Comprehensive client and server-side error management

### 📁 **Template Files (Located in `/templates/upload/`)**

- `page.html` - Standalone upload page (integrated with template engine)
- `form.html` - Reusable upload form component
- `styles.css` - Professional styling matching IronDrop theme  
- `script.js` - Upload functionality with drag-drop and progress tracking

### 🔗 **Integration Points**

- **Template Engine**: `src/templates.rs` includes upload template rendering methods
- **HTTP Handler**: `src/http.rs` serves upload pages and processes uploads
- **Upload Handler**: `src/upload.rs` handles file processing with multipart parsing
- **CLI Configuration**: Upload settings integrated with command-line options

### 🧪 **Test Coverage**

- **Upload Integration Tests**: 29 comprehensive test cases in `tests/upload_integration_test.rs`
- **UI Template Tests**: Template rendering and static asset serving validated
- **Security Tests**: Upload validation and error handling thoroughly tested
- **End-to-End Tests**: Complete upload workflow testing from UI to file system

### 🎨 **Design System**

The upload UI seamlessly integrates with IronDrop's design language:
- **Color Scheme**: Professional blackish-grey palette (#0a0a0a to #ffffff)
- **Typography**: Consistent with directory listing and error pages
- **Interactions**: Smooth animations and hover effects
- **Responsive**: Mobile-friendly design for all device types

This upload UI system provides a modern, accessible, and performant file upload experience that seamlessly integrates with IronDrop's existing design system and is ready for production use.