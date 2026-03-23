import { test, expect } from '@playwright/test';

test.describe('Phase 3C — Zoom Level Testing', () => {
  test('200% zoom — check for layout overflow', async ({ browser }) => {
    // Simulate 200% zoom by halving viewport at 2x scale
    const context = await browser.newContext({
      viewport: { width: 640, height: 360 },
      deviceScaleFactor: 2,
    });
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    const overflows = await page.evaluate(() => {
      const issues: Array<{ el: string; scrollW: number; clientW: number; scrollH: number; clientH: number }> = [];
      const elements = document.querySelectorAll('*');
      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        if (htmlEl.scrollWidth > htmlEl.clientWidth + 2 || htmlEl.scrollHeight > htmlEl.clientHeight + 50) {
          // Skip body/html and elements with intentional scroll
          const style = window.getComputedStyle(htmlEl);
          if (style.overflow === 'auto' || style.overflow === 'scroll' ||
              style.overflowX === 'auto' || style.overflowX === 'scroll' ||
              style.overflowY === 'auto' || style.overflowY === 'scroll') continue;
          if (htmlEl.tagName === 'HTML' || htmlEl.tagName === 'BODY') continue;

          issues.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 50)}`,
            scrollW: htmlEl.scrollWidth,
            clientW: htmlEl.clientWidth,
            scrollH: htmlEl.scrollHeight,
            clientH: htmlEl.clientHeight,
          });
        }
      }
      return issues;
    });

    console.log(`\n=== ZOOM 200% OVERFLOW CHECK ===`);
    console.log(`Viewport: 640×360 @ 2x scale`);
    if (overflows.length === 0) {
      console.log('✅ No overflow issues detected');
    } else {
      console.log(`⚠️ ${overflows.length} elements overflow:`);
      for (const o of overflows.slice(0, 15)) {
        const xOverflow = o.scrollW > o.clientW + 2 ? `X: ${o.scrollW - o.clientW}px` : '';
        const yOverflow = o.scrollH > o.clientH + 50 ? `Y: ${o.scrollH - o.clientH}px` : '';
        console.log(`  ${o.el}: overflow ${[xOverflow, yOverflow].filter(Boolean).join(', ')}`);
      }
      if (overflows.length > 15) console.log(`  ... and ${overflows.length - 15} more`);
    }

    // Screenshot at 200% zoom
    await page.screenshot({ path: 'docs/audit/screenshot-zoom-200.png', fullPage: true });
    console.log('Screenshot saved: docs/audit/screenshot-zoom-200.png');

    await context.close();
  });

  test('400% zoom — check for layout overflow', async ({ browser }) => {
    const context = await browser.newContext({
      viewport: { width: 320, height: 180 },
      deviceScaleFactor: 4,
    });
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    const overflows = await page.evaluate(() => {
      const issues: Array<{ el: string; scrollW: number; clientW: number; overflow: string }> = [];
      const elements = document.querySelectorAll('*');
      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        const style = window.getComputedStyle(htmlEl);
        // Only check horizontal overflow (vertical scroll is expected)
        if (htmlEl.scrollWidth > htmlEl.clientWidth + 2) {
          if (style.overflow === 'auto' || style.overflow === 'scroll' ||
              style.overflowX === 'auto' || style.overflowX === 'scroll' ||
              style.overflowX === 'hidden') continue;
          if (htmlEl.tagName === 'HTML' || htmlEl.tagName === 'BODY') continue;

          issues.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 50)}`,
            scrollW: htmlEl.scrollWidth,
            clientW: htmlEl.clientWidth,
            overflow: style.overflow,
          });
        }
      }
      return issues;
    });

    console.log(`\n=== ZOOM 400% OVERFLOW CHECK ===`);
    console.log(`Viewport: 320×180 @ 4x scale`);
    if (overflows.length === 0) {
      console.log('✅ No horizontal overflow issues');
    } else {
      console.log(`⚠️ ${overflows.length} elements with horizontal overflow:`);
      for (const o of overflows.slice(0, 15)) {
        console.log(`  ${o.el}: overflow ${o.scrollW - o.clientW}px (overflow: ${o.overflow})`);
      }
    }

    await page.screenshot({ path: 'docs/audit/screenshot-zoom-400.png', fullPage: true });
    console.log('Screenshot saved: docs/audit/screenshot-zoom-400.png');

    await context.close();
  });

  test('minimum viewport 320px — WCAG 1.4.10 Reflow', async ({ browser }) => {
    // WCAG 1.4.10: Content must reflow at 320px width without horizontal scroll
    const context = await browser.newContext({
      viewport: { width: 320, height: 568 },
    });
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    const hasHScroll = await page.evaluate(() => {
      return document.documentElement.scrollWidth > document.documentElement.clientWidth;
    });

    const truncated = await page.evaluate(() => {
      const elements = document.querySelectorAll('*');
      const results: string[] = [];
      for (const el of elements) {
        const style = window.getComputedStyle(el);
        if (style.textOverflow === 'ellipsis' && (el as HTMLElement).offsetWidth > 0) {
          const text = (el as HTMLElement).innerText?.substring(0, 40);
          if (text) results.push(`${el.tagName.toLowerCase()}: "${text}"`);
        }
      }
      return results;
    });

    console.log(`\n=== 320px REFLOW (WCAG 1.4.10) ===`);
    console.log(`Horizontal scroll present: ${hasHScroll ? '❌ YES' : '✅ NO'}`);
    if (truncated.length > 0) {
      console.log(`Text truncated with ellipsis (${truncated.length}):`);
      truncated.forEach(t => console.log(`  ${t}`));
    }

    await page.screenshot({ path: 'docs/audit/screenshot-320px.png', fullPage: true });

    await context.close();
  });
});
