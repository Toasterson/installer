# Illumos Installer UI - Architecture Overview

## Introduction

The Illumos Installer UI is a modern, modular Rust application built with Dioxus that provides a step-by-step guided interface for installing illumos on target machines. This document outlines the architectural decisions, module structure, and design patterns used throughout the application.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Dioxus App                           │
├─────────────────────────────────────────────────────────┤
│                 Router & Navigation                     │
├─────────────────────────────────────────────────────────┤
│                    UI Components                        │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────────┐  │
│  │   Layout    │ │    Pages     │ │   Reusable       │  │
│  │ Components  │ │  Components  │ │   Components     │  │
│  └─────────────┘ └──────────────┘ └──────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                 State Management                        │
│           (Centralized InstallerState)                 │
├─────────────────────────────────────────────────────────┤
│                Server Functions                         │
│        (Communication with machined)                   │
├─────────────────────────────────────────────────────────┤
│              External Dependencies                      │
│    machineconfig │ sysconfig │ reqwest │ tokio         │
└─────────────────────────────────────────────────────────┘
```

## Module Structure

### `main.rs` - Application Entry Point
- **Purpose**: Bootstraps the Dioxus application
- **Responsibilities**:
  - Initialize global state context
  - Load CSS and favicon assets
  - Set up the main Router component
- **Dependencies**: All modules (components, routes, state, server)

### `state/` - Application State Management
- **Purpose**: Centralized state management and data structures
- **Key Components**:
  - `InstallerState`: Main application state struct
  - Data models: `MachineServer`, `Pool`, `VDev`, `NetworkInterface`, etc.
  - Validation helpers and utility methods
- **Design Pattern**: Single source of truth with reactive signals
- **State Flow**:
  ```
  User Input → Component → Update State → Reactive Re-render
  ```

### `routes/` - Navigation and Routing
- **Purpose**: Define application routes and navigation flow
- **Key Components**:
  - `Route` enum with all application pages
  - Navigation helpers (next/previous route logic)
  - Step validation and progress tracking
  - Route metadata (display names, icons, step numbers)
- **Design Pattern**: Type-safe routing with compile-time verification

### `components/` - UI Component Library
Organized into two main categories:

#### `layout.rs` - Structural Components
- **MainLayout**: App shell with header, main content, and footer
- **NavigationButtons**: Step-by-step navigation controls
- **ProgressBreadcrumb**: Visual progress indicator
- **AppHeader**: Application title and progress bar

#### `pages/` - Page Components
Each page component follows a consistent pattern:
- **Input validation**: Real-time validation feedback
- **State synchronization**: Two-way data binding with global state
- **Error handling**: User-friendly error states
- **Responsive design**: Adaptive layouts for different screen sizes

Page breakdown:
- `welcome.rs`: Introduction and overview
- `server_selection.rs`: Machine discovery and selection
- `storage_configuration.rs`: ZFS pool and vdev configuration
- `network_configuration.rs`: Network interface setup
- `system_configuration.rs`: System settings (hostname, timezone, etc.)
- `review_configuration.rs`: Configuration summary and validation
- `installation.rs`: Installation progress and monitoring

### `server/` - Backend Communication
- **Purpose**: Handle all communication with machined servers
- **Key Functions**:
  - `load_available_servers()`: Discover available machines
  - `claim_server()`: Reserve a machine for installation
  - `perform_installation()`: Execute the installation process
  - `convert_to_machine_config()`: Transform UI state to machine config
- **Design Pattern**: Server functions with async/await for non-blocking operations

## Design Patterns and Principles

### 1. Component-Based Architecture
- **Modularity**: Each component has a single responsibility
- **Reusability**: Common UI patterns are extracted into reusable components
- **Composition**: Complex UIs built by composing smaller components

### 2. Reactive State Management
- **Centralized State**: Single `InstallerState` context shared across components
- **Reactive Updates**: Components automatically re-render when state changes
- **Immutable Updates**: State modifications create new state to ensure consistency

### 3. Type Safety
- **Strong Typing**: Rust's type system prevents runtime errors
- **Enum-based Routing**: Compile-time route verification
- **Structured Data**: Well-defined data models for all configuration types

### 4. Separation of Concerns
- **UI Logic**: Components focus on presentation and user interaction
- **Business Logic**: State management handles validation and data transformation
- **Communication**: Server functions handle external system integration
- **Navigation**: Routes module manages application flow

### 5. Error Handling
- **Result Types**: Proper error propagation with `Result<T, E>`
- **User-Friendly Messages**: Errors are transformed into actionable user feedback
- **Recovery Mechanisms**: Retry functionality for transient failures

## Data Flow

### 1. User Input Flow
```
User Action → Component Event Handler → State Update → UI Re-render
```

### 2. Server Communication Flow
```
Component → Server Function → External API → Response → State Update → UI Update
```

### 3. Navigation Flow
```
User Navigation → Route Change → Component Mount → State Validation → UI Render
```

## State Management Strategy

### Global State Structure
```rust
InstallerState {
    // Server selection
    selected_server: Option<String>,
    server_list: Vec<MachineServer>,
    
    // Storage configuration
    pools: Vec<Pool>,
    image: String,
    boot_environment_name: Option<String>,
    
    // Network configuration
    hostname: String,
    nameservers: Vec<String>,
    interfaces: Vec<NetworkInterface>,
    
    // Installation progress
    installation_progress: f32,
    installation_log: Vec<String>,
}
```

### State Update Patterns
1. **Direct Updates**: Simple field modifications
2. **Batch Updates**: Multiple related changes in single operation
3. **Validation**: State changes trigger validation logic
4. **Persistence**: Critical state changes can be serialized

## Component Communication

### Parent-Child Communication
- **Props**: Data flows down through component properties
- **Event Handlers**: Child components notify parents through callbacks

### Sibling Communication
- **Shared State**: Components communicate through shared context
- **Event Bus**: For complex multi-component interactions (if needed)

### Global Communication
- **Context API**: Global state accessible from any component
- **Server Functions**: Async operations for external communication

## Performance Considerations

### Rendering Optimization
- **Reactive Signals**: Only re-render components when relevant state changes
- **Component Splitting**: Break large components into smaller, focused pieces
- **Lazy Loading**: Load heavy components only when needed

### Memory Management
- **Rust Ownership**: Automatic memory management without garbage collection
- **Clone Minimization**: Efficient state sharing to reduce memory overhead
- **Resource Cleanup**: Proper cleanup of async operations and event handlers

## Security Considerations

### Input Validation
- **Client-Side**: Immediate feedback for user experience
- **Server-Side**: Authoritative validation before processing
- **Type Safety**: Rust prevents many common vulnerabilities

### State Protection
- **Immutable State**: Prevents accidental state corruption
- **Validation Gates**: State changes go through validation layer
- **Error Boundaries**: Graceful handling of unexpected errors

## Testing Strategy

### Component Testing
- **Unit Tests**: Individual component behavior
- **Integration Tests**: Component interaction with state
- **Snapshot Tests**: UI regression testing

### State Testing
- **State Transitions**: Validate state changes
- **Validation Logic**: Ensure proper input validation
- **Error Conditions**: Test error handling paths

### End-to-End Testing
- **User Flows**: Complete installation workflows
- **Cross-Platform**: Testing on different target platforms
- **Performance**: Load testing and responsiveness

## Development Guidelines

### Code Organization
1. **One component per file**: Clear module boundaries
2. **Consistent naming**: Follow Rust naming conventions
3. **Documentation**: Document public APIs and complex logic
4. **Error Handling**: Always handle potential failure cases

### State Management
1. **Minimal State**: Keep only necessary data in global state
2. **Immutable Updates**: Use proper Rust patterns for state changes
3. **Validation**: Validate state changes at appropriate boundaries
4. **Serialization**: Ensure state can be serialized for persistence

### UI Development
1. **Responsive Design**: Support multiple screen sizes
2. **Accessibility**: Follow web accessibility standards
3. **User Experience**: Provide clear feedback and error messages
4. **Performance**: Optimize for smooth interactions

## Future Considerations

### Scalability
- **Plugin Architecture**: Support for custom installation steps
- **Theme System**: Customizable UI themes
- **Internationalization**: Multi-language support

### Enhanced Features
- **Real-time Updates**: WebSocket integration for live progress
- **Configuration Templates**: Pre-defined installation templates
- **Advanced Validation**: Cross-field validation and dependencies
- **Backup/Restore**: Configuration backup and restore functionality

### Platform Support
- **Mobile Optimization**: Enhanced mobile experience
- **Offline Support**: Cache for offline installation scenarios
- **Performance Monitoring**: Built-in performance metrics

This architecture provides a solid foundation for the illumos installer UI while maintaining flexibility for future enhancements and requirements.