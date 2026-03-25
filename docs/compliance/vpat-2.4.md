# Voluntary Product Accessibility Template (VPAT) 2.4

## WCAG 2.1 Edition

**Product:** Reasonance IDE
**Version:** 1.0.2
**Date:** 2026-03-25
**Contact:** https://github.com/TNASRLSB/reasonance/issues
**Evaluation methods:** Source code review, component-level WCAG 2.1 matrix audit, manual testing
**Applicable standard:** WCAG 2.1 Level AA

---

## Terms

- **Supports:** The functionality of the product has at least one method that meets the criterion without known defects or meets with equivalent facilitation.
- **Partially Supports:** Some functionality of the product does not meet the criterion.
- **Does Not Support:** The majority of product functionality does not meet the criterion.
- **Not Applicable:** The criterion is not relevant to the product.

---

## Table 1: Success Criteria, Level A

| Criteria | Conformance Level | Remarks and Explanations |
|----------|-------------------|--------------------------|
| **1.1.1 Non-text Content** | Partially Supports | Most components provide `aria-label` on non-text controls. DiffView accept/reject buttons lack explicit `aria-label`. |
| **1.2.1 Audio-only and Video-only (Prerecorded)** | Not Applicable | No prerecorded audio or video content. |
| **1.2.2 Captions (Prerecorded)** | Not Applicable | No prerecorded multimedia. |
| **1.2.3 Audio Description or Media Alternative (Prerecorded)** | Not Applicable | No prerecorded multimedia. |
| **1.3.1 Info and Relationships** | Partially Supports | Semantic landmarks, ARIA tree widget, tablists, dialog roles used throughout. MenuItem uses `role="menubar"` on individual items instead of parent bar (incorrect ARIA structure). |
| **1.3.2 Meaningful Sequence** | Supports | DOM order matches visual layout across all components. |
| **1.3.3 Sensory Characteristics** | Supports | Instructions do not rely solely on sensory characteristics. |
| **1.4.1 Use of Color** | Supports | Status indicators use text labels alongside color. DiffBlock +/- prefix spans enlarged and no longer `aria-hidden`, providing a non-color distinction. YOLO mode retains `aria-pressed` state. |
| **1.4.2 Audio Control** | Not Applicable | No auto-playing audio. |
| **2.1.1 Keyboard** | Supports | Global shortcuts, focus traps in dialogs, searchable command palette. FileTree keyboard navigation (tabindex, Enter/Space, arrows). MenuItem submenus keyboard-accessible (ArrowRight/Left). Toast dismiss reachable via keyboard (tabindex, focus pauses timer). |
| **2.1.2 No Keyboard Trap** | Supports | Dialogs implement focus trap with Escape exit. ResponsePanel has focus trap and Escape close. |
| **2.1.4 Character Key Shortcuts** | Supports | No single-character shortcuts without modifier keys. |
| **2.2.1 Timing Adjustable** | Partially Supports | Error and warning toasts are persistent (never auto-dismiss). Info/success toasts auto-dismiss after 5 seconds; focus pauses the timer but there is no mechanism to extend or disable the duration. |
| **2.3.1 Three Flashes or Below Threshold** | Supports | Application uses `prefers-reduced-motion: reduce` blanket override (8+ implementations). No flashing content. |
| **2.4.1 Bypass Blocks** | Supports | Skip links to file tree, editor, and terminal. Command palette for direct navigation. |
| **2.4.2 Page Titled** | Supports | Application window title identifies the product. |
| **2.4.3 Focus Order** | Supports | Logical focus order throughout. FileTree keyboard-reachable. EditorTabs and TerminalManager implement roving tabindex with ArrowLeft/Right navigation. ResponsePanel focus trap added. |
| **2.4.4 Link Purpose (In Context)** | Supports | Links and buttons have descriptive text or labels. |
| **2.5.1 Pointer Gestures** | Supports | All functionality available via single-point activation. Panel resizing uses drag but also keyboard arrows. |
| **2.5.2 Pointer Cancellation** | Supports | Standard browser button activation (mouseup). |
| **2.5.3 Label in Name** | Partially Supports | Most visible labels match accessible names. YOLO button uses "YOLO" text without descriptive `aria-label`. |
| **2.5.4 Motion Actuation** | Not Applicable | No motion-activated functionality. |
| **3.1.1 Language of Page** | Supports | HTML `lang` attribute set. i18n system supports 9 languages. |
| **3.2.1 On Focus** | Supports | No unexpected context changes on focus across all components. |
| **3.2.2 On Input** | Supports | All context changes are user-initiated. Settings warns about YOLO mode restart via tooltip. |
| **3.3.1 Error Identification** | Partially Supports | Error boundaries, `role="alert"` on error banners, ErrorBlock for chat errors. FileTree silently catches directory listing errors. |
| **3.3.2 Labels or Instructions** | Supports | Form labels with `for` attribute in Settings. Terminal container and search input have `aria-label`. DiffBlock has `aria-expanded` and `aria-label`. All 60+ previously hardcoded English strings replaced with i18n keys across 9 languages. |
| **4.1.1 Parsing** | Supports | Well-formed HTML throughout. Minor semantic issues (MenuItem `role="menubar"` placement) but parseable. |
| **4.1.2 Name, Role, Value** | Supports | Strong ARIA coverage throughout. Terminal container has `role="tabpanel"` and `aria-label`. DiffBlock has `aria-expanded` and `aria-label`. ResponsePanel has `role="dialog"` and `aria-label`. TerminalManager add-session button has i18n `aria-label`. |

---

## Table 2: Success Criteria, Level AA

| Criteria | Conformance Level | Remarks and Explanations |
|----------|-------------------|--------------------------|
| **1.3.4 Orientation** | Supports | Content adapts to portrait and landscape. No orientation lock. |
| **1.3.5 Identify Input Purpose** | Not Applicable | No form fields collecting personal information with standard autocomplete purposes. |
| **1.4.3 Contrast (Minimum)** | Supports | Body text ~12.3:1. All `--accent` text-on-dark usages now use `--accent-text` (7.1:1, AAA). ResponsePanel/MarkdownPreview links updated. StatusBar uses `--accent-statusbar: #1e40af` (8.59:1); element opacity raised to 0.75. |
| **1.4.4 Resize Text** | Supports | Flexible layouts with scrollable areas. Minor truncation in FileTree and EditorTabs at larger sizes. |
| **1.4.5 Images of Text** | Supports | No images of text. All text rendered as actual text. |
| **1.4.10 Reflow** | Supports | Responsive layout with resizable panels. No horizontal scrolling required at 320px width. |
| **1.4.11 Non-text Contrast** | Partially Supports | Most UI components meet 3:1. ResponsePanel close button lacks distinct visual boundary. |
| **1.4.12 Text Spacing** | Supports | Base `line-height: 1.5`. Enhanced Readability mode adds `letter-spacing: 0.05em`. Minor clipping possible in FileTree/EditorTabs with `overflow: hidden`. |
| **1.4.13 Content on Hover or Focus** | Partially Supports | Browser-managed tooltips are dismissible. MenuItem submenus open only on mouse hover, not hoverable by keyboard, disappear on mouse leave. |
| **2.4.5 Multiple Ways** | Supports | File tree navigation, command palette (Ctrl+P), editor tabs, search palette, find in files. |
| **2.4.6 Headings and Labels** | Partially Supports | Descriptive headings and labels in most components. Terminal and ResponsePanel lack `aria-label`. |
| **2.4.7 Focus Visible** | Partially Supports | Focus rings on most interactive elements. Terminal search buttons may lack visible focus ring at small sizes. |
| **2.5.8 Target Size (Minimum)** | Partially Supports | Primary close/dismiss buttons fixed to `min-width`/`min-height: 32px` (ShortcutsDialog, FindInFiles, ResponsePanel, HiveCanvas) and 24px (EditorTabs tab-save). Some borderline controls remain at 24px, which meets the AA minimum but not AAA. |
| **3.1.2 Language of Parts** | Supports | i18n system handles language switching. Individual component language changes not needed (single-language UI per session). |
| **3.2.3 Consistent Navigation** | Supports | Navigation components (Toolbar, StatusBar, FileTree) appear consistently across all views. |
| **3.2.4 Consistent Identification** | Supports | Icons, labels, and controls used consistently throughout. |
| **3.3.3 Error Suggestion** | Partially Supports | Error banners suggest retry actions. FileTree errors not surfaced. No structured suggestions for invalid input. |
| **3.3.4 Error Prevention (Legal, Financial, Data)** | Not Applicable | No legal, financial, or data submission transactions. |
| **4.1.3 Status Messages** | Partially Supports | `aria-live` regions for chat messages, streaming indicator, toast notifications, status bar. FileTree lacks loading/error status messages. ResponsePanel appearance not announced. |

---

## Top Known Issues

All previously blocking issues have been resolved. Remaining minor items:

1. **Target size** — some borderline controls remain at exactly 24px (meets AA minimum; does not achieve AAA 44px target)
2. **FileTree errors** — directory listing errors are silently caught; no accessible status announcement on failure
3. **Toast auto-dismiss** — info/success toasts auto-dismiss after 5 seconds; focus pauses the timer but no user-configurable mechanism to extend or disable (error/warning toasts are persistent)

---

## Legal Disclaimer

This VPAT is provided for informational purposes and represents a self-assessment of accessibility conformance based on internal code review, component-level WCAG 2.1 matrix audit, and manual testing. It is not a certification of accessibility. Third-party testing is recommended before making formal conformance claims. This document reflects the state of Reasonance IDE version 1.0.2 as of 2026-03-25 and may not reflect subsequent updates.
