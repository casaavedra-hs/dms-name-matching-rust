# v12 Design Tokens

## Fonts

```css
--font-sans: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

## Colors

```css
--color-bg: #F6F8FC;
--color-surface: #FFFFFF;
--color-text: #132238;
--color-muted: #64748B;
--color-border: #E2E8F0;
--color-primary: #1E3A8A;
--color-primary-soft: #DBEAFE;
--color-success: #166534;
--color-success-soft: #DCFCE7;
--color-warning: #92400E;
--color-warning-soft: #FEF3C7;
--color-danger: #991B1B;
--color-danger-soft: #FEE2E2;
--color-purple-soft: #EDE9FE;
--color-rose-soft: #FFE4E6;
```

## Card Style

```css
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 20px;
  box-shadow: 0 10px 30px rgba(15, 23, 42, 0.06);
}
```

## Status Chips

- Matched: green
- Fuzzy: purple
- Exact: blue
- Ambiguous: amber
- Unmatched: slate
- Failed: rose/red

## Layout

- Sidebar width: 280px
- Content max width: 1600px
- Card gap: 20px
- Base spacing: 8px scale
- Buttons: 12px vertical, 16px horizontal
- Input height: 44px

## UI Tone

Simple, premium, enterprise. Avoid noisy gradients and crowded tables. Use pastel cards for separation, not decoration.
