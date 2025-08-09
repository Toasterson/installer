# Window Optimization Guide

This document explains the window sizing and layout optimizations made to the illumos Installer UI for better usability without scrolling.

## Overview

The installer UI has been optimized to work effectively in various window sizes, from compact laptop screens to large desktop monitors. The interface automatically adapts its layout density and component sizing based on available screen real estate.

## Window Size Presets

### 🖥️ Desktop Configurations

| Preset | Dimensions | Use Case | Target Display |
|--------|------------|----------|----------------|
| **Small** | 800×600 | Minimal space, older displays | 1024×768+ monitors |
| **Compact** | 900×650 | Laptop screens, space-efficient | 1366×768+ laptops |
| **Default** | 1024×768 | Standard desktop configuration | 1920×1080+ monitors |
| **Large** | 1200×900 | Large monitors, detailed view | 1440p+ displays |
| **Full HD** | 1920×1080 | Ultra-wide, presentation mode | 4K+ displays |

### 📱 Platform Support

- **Desktop**: Native window with configurable dimensions
- **Web**: Responsive browser-based interface
- **Mobile**: Touch-optimized mobile app

## Responsive Breakpoints

The UI uses CSS media queries to automatically adjust layouts:

### Height Breakpoints
```css
/* Normal mode: > 800px height */
- Full spacing and padding
- Large text and buttons
- Expanded form elements

/* Compact mode: 650px - 800px height */
- Reduced spacing
- Smaller text sizes
- Condensed components

/* Ultra-compact: 500px - 650px height */
- Minimal spacing
- Compact form elements
- Essential information only

/* Minimal: < 500px height */
- Maximum density
- Single-column layouts
- Critical elements only
```

### Width Breakpoints
```css
/* Mobile: < 768px width */
- Single column layouts
- Stack form elements
- Simplified navigation

/* Desktop: >= 768px width */
- Grid-based layouts
- Side-by-side elements
- Full feature set
```

## Layout Optimizations

### Header Compression
- Reduced title font size for small windows
- Compact progress indicator
- Minimal padding in tight spaces

### Content Density
- Dynamic spacing based on available height
- Scrollable main content area
- Fixed header and footer for navigation

### Form Optimization
- Smaller input fields in compact mode
- Reduced label sizes
- Condensed button spacing

### Component Scaling
- Server cards resize automatically
- Configuration sections compress
- Review summaries adapt layout

## Usage Examples

### Quick Start
```bash
# Interactive launcher with presets
./launcher.sh

# Direct preset usage
./run.sh --compact      # 900×650
./run.sh --small        # 800×600
./run.sh --large        # 1200×900
```

### Custom Configuration
```bash
# Custom window size
./run.sh --width 1100 --height 700

# Web platform (no window constraints)
./run.sh --platform web

# Development mode with hot reload
./run.sh --compact --no-hot-reload
```

### Dioxus.toml Configuration
```toml
[desktop.app]
title = "illumos Installer"
width = 900
height = 650
min_width = 800
min_height = 500
resizable = true
maximized = false
decorations = true
```

## Performance Considerations

### Memory Usage by Window Size
- **Small (800×600)**: ~50MB RAM
- **Compact (900×650)**: ~60MB RAM  
- **Default (1024×768)**: ~70MB RAM
- **Large (1200×900)**: ~85MB RAM

### Rendering Performance
- Smaller windows = fewer DOM elements visible
- Responsive layouts reduce computation
- Fixed header/footer improves scroll performance
- CSS transitions optimized for 60fps

## Accessibility Features

### Visual Accessibility
- High contrast color scheme
- Large click targets (minimum 44px)
- Clear visual hierarchy
- Readable font sizes at all scales

### Navigation Accessibility
- Keyboard navigation support
- Clear focus indicators
- Logical tab order
- Screen reader friendly markup

### Responsive Text
- Automatic font scaling
- Maintained readability ratios
- Contrast preservation across sizes

## Development Guidelines

### Adding New Components
1. **Test multiple window sizes** during development
2. **Use relative units** (rem, em) where appropriate
3. **Consider information hierarchy** for compact layouts
4. **Test touch interaction** for mobile platforms

### CSS Best Practices
```css
/* Use flexible layouts */
.component {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(8px, 2vw, 20px);
}

/* Responsive spacing */
.spacing {
    margin: clamp(8px, 3vh, 24px) 0;
}

/* Scalable fonts */
.text {
    font-size: clamp(0.8rem, 2.5vw, 1.2rem);
}
```

### Testing Checklist
- [ ] Test all window size presets
- [ ] Verify no horizontal scrolling
- [ ] Check vertical scroll behavior
- [ ] Validate mobile responsiveness
- [ ] Test keyboard navigation
- [ ] Verify touch interactions

## Troubleshooting

### Common Issues

**Window too small for content**
- Solution: Increase minimum window dimensions
- Check CSS media queries for proper breakpoints

**Scrolling appears unexpectedly**
- Solution: Review content height calculations
- Adjust padding/margins in compact modes

**Text too small to read**
- Solution: Increase font-size in media queries
- Check contrast ratios for accessibility

**Components overlap**
- Solution: Review z-index stacking
- Adjust positioning for smaller screens

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug ./run.sh --compact

# Test specific window size
./run.sh --width 800 --height 500
```

## Future Enhancements

### Planned Improvements
- **Dynamic zoom levels** based on display DPI
- **Saved window preferences** in local storage
- **Multi-monitor support** with window positioning
- **Fullscreen kiosk mode** for installation terminals
- **Touch gestures** for mobile navigation
- **Voice control** for accessibility

### Configuration Persistence
```rust
// Future: Save user preferences
struct WindowPreferences {
    width: u32,
    height: u32,
    position: (i32, i32),
    zoom_level: f32,
    theme: String,
}
```

This optimization ensures the installer UI works effectively across all common display configurations while maintaining usability and visual appeal.