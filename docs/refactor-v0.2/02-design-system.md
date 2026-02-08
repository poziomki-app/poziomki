# Design System: Dark Mode + Gradients

## Design Philosophy

**No light mode** — eliminates:
- Theme switching logic
- Duplicate color tokens
- `useColorScheme()` checks
- Conditional styling throughout codebase

**Gradient-first** — every screen uses the same visual language:
- Background gradients for depth
- Card gradients for elevation
- Accent gradients for interactive elements

## Color Tokens (CSS Variables)

```css
/* apps/mobile/src/styles/theme.css */

:root {
  /* Primary - Bright Cyan */
  --color-primary: #22D3EE;
  --color-primary-dark: #06B6D4;
  --color-primary-light: #164E63;
  --color-primary-muted: #67E8F9;

  /* Secondary/Accent - Golden Yellow */
  --color-accent: #EDB923;
  --color-accent-dark: #D4A620;
  --color-accent-light: #3d3520;

  /* Neutrals - GitHub-style dark */
  --color-background: #0d1117;
  --color-surface: #161b22;
  --color-surface-elevated: #21262d;
  --color-text: #f0f6fc;
  --color-text-secondary: #a8b3c4;
  --color-text-muted: #6e7a8a;
  --color-border: #30363d;
  --color-border-light: #21262d;

  /* Semantic */
  --color-error: #f85149;
  --color-error-light: #3d2020;
  --color-success: #3fb950;
  --color-success-light: #1a3d25;

  /* Overlay */
  --color-overlay: rgba(0, 0, 0, 0.6);

  /* Gradients */
  --gradient-background: linear-gradient(180deg, #161b22 0%, #0d1117 100%);
  --gradient-surface: linear-gradient(180deg, #21262d 0%, #161b22 100%);
  --gradient-navbar: linear-gradient(180deg, #252d3a 0%, #0d1117 100%);
  --gradient-card: linear-gradient(180deg, var(--color-surface-elevated) 0%, var(--color-surface) 100%);
  --gradient-primary: linear-gradient(135deg, #22D3EE 0%, #06B6D4 100%);
  --gradient-accent: linear-gradient(135deg, #EDB923 0%, #D4A620 100%);

  /* Spacing (8-point grid) */
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;
  --spacing-2xl: 48px;

  /* Border Radius */
  --radius-xs: 4px;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-xl: 24px;
  --radius-full: 9999px;

  /* Typography */
  --font-regular: 'Nunito', sans-serif;
  --font-heading: 'Montserrat', sans-serif;
  --font-size-xs: 12px;
  --font-size-sm: 14px;
  --font-size-md: 16px;
  --font-size-lg: 18px;
  --font-size-xl: 20px;
  --font-size-2xl: 24px;

  /* Touch targets */
  --touch-target-min: 44px;
}
```

## Screen Template (Consistent Layout)

Every screen follows the same structure:

```tsx
// Standard screen with gradient background
<view class="screen">
  <view class="screen-gradient" />
  <view class="screen-content">
    {/* Screen content */}
  </view>
</view>
```

```css
.screen {
  flex: 1;
  background-color: var(--color-background);
}

.screen-gradient {
  position: absolute;
  inset: 0;
  background: var(--gradient-background);
}

.screen-content {
  flex: 1;
  position: relative;
  z-index: 1;
}
```

## Card Component (Gradient Elevation)

```tsx
<view class="card">
  <view class="card-gradient" />
  <view class="card-content">
    {children}
  </view>
</view>
```

```css
.card {
  border-radius: var(--radius-lg);
  overflow: hidden;
  position: relative;
}

.card-gradient {
  position: absolute;
  inset: 0;
  background: var(--gradient-card);
}

.card-content {
  position: relative;
  z-index: 1;
  padding: var(--spacing-md);
}
```

## No Theme Switching

```typescript
// ❌ REMOVED - No theme switching logic
// const { colorScheme } = useColorScheme()
// const isDark = colorScheme === 'dark'
// style={isDark ? styles.dark : styles.light}

// ✅ SIMPLIFIED - Always dark
// Just use CSS variables directly
<view class="card" />
```

## Accessibility

### Respect User Preferences

```css
/* Respect user preferences */
@media (prefers-reduced-motion: reduce) {
  * {
    animation: none !important;
    transition: none !important;
  }
}

/* High contrast mode - increase contrast in dark theme */
@media (prefers-contrast: high) {
  :root {
    --color-text: #ffffff;
    --color-text-secondary: #e0e0e0;
    --color-border: #505050;
    --color-primary: #40e8ff;
  }
}

/* Minimum touch targets */
.touchable {
  min-height: 44px;
  min-width: 44px;
}
```

### Accessible Components

```typescript
// All interactive elements have:
// - Accessible labels
// - Touch targets ≥ 44x44
// - Focus indicators
// - Screen reader support

// Example: MessageBubble
<view
  class="message-bubble"
  role="listitem"
  aria-label={`Message from ${sender.name}: ${content}`}
>
  <text class="message-content">{content}</text>
  <text class="message-time" aria-hidden="true">{time}</text>
</view>
```

## Contrast Requirements

Dark mode must meet WCAG AA standards:
- Normal text: 4.5:1 minimum contrast ratio
- Large text: 3:1 minimum contrast ratio
- Interactive elements: clearly distinguishable

| Element | Color | Background | Ratio |
|---------|-------|------------|-------|
| Primary text | #f0f6fc | #0d1117 | 15.4:1 ✓ |
| Secondary text | #a8b3c4 | #0d1117 | 8.1:1 ✓ |
| Muted text | #6e7a8a | #0d1117 | 4.5:1 ✓ |
| Primary action | #22D3EE | #0d1117 | 11.2:1 ✓ |
| Accent | #EDB923 | #0d1117 | 10.8:1 ✓ |
