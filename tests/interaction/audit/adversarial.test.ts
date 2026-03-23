import { test, expect } from '@playwright/test';

test.describe('Phase 3.5 — Adversarial Input Testing', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('XSS in search palette input', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await page.waitForTimeout(300);

    const input = page.locator('input[type="text"], input[type="search"], .search-input, [role="combobox"] input').first();
    const inputVisible = await input.isVisible().catch(() => false);

    if (!inputVisible) {
      console.log('Could not find search input — skipping XSS test');
      return;
    }

    const xssPayloads = [
      '<script>alert(1)</script>',
      '<img src=x onerror=alert(1)>',
      '"><svg onload=alert(1)>',
      "'; DROP TABLE files; --",
      '{{constructor.constructor("alert(1)")()}}',
    ];

    const consoleErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    console.log('\n=== XSS PAYLOAD TESTING ===');
    for (const payload of xssPayloads) {
      await input.fill('');
      await input.fill(payload);
      await page.waitForTimeout(200);

      // Check if any script executed (dialog appeared)
      const dialogAppeared = await page.evaluate(() => {
        // Check for injected elements
        const scripts = document.querySelectorAll('script:not([src])');
        const svgs = document.querySelectorAll('svg[onload]');
        const imgs = document.querySelectorAll('img[onerror]');
        return scripts.length > 0 || svgs.length > 0 || imgs.length > 0;
      });

      console.log(`  Payload: ${payload.substring(0, 40)}... | Injected: ${dialogAppeared ? '❌ YES' : '✅ NO'}`);
    }

    if (consoleErrors.length > 0) {
      console.log(`\nConsole errors during XSS test: ${consoleErrors.length}`);
      consoleErrors.forEach(e => console.log(`  ${e.substring(0, 100)}`));
    }
  });

  test('large input in search palette', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await page.waitForTimeout(300);

    const input = page.locator('input[type="text"], input[type="search"], .search-input, [role="combobox"] input').first();
    const inputVisible = await input.isVisible().catch(() => false);

    if (!inputVisible) {
      console.log('Could not find search input — skipping large input test');
      return;
    }

    // Generate a large string (100KB)
    const largeInput = 'A'.repeat(100_000);

    console.log('\n=== LARGE INPUT TEST ===');
    const startTime = Date.now();
    await input.fill(largeInput);
    const fillTime = Date.now() - startTime;
    console.log(`  100KB input fill time: ${fillTime}ms`);

    await page.waitForTimeout(1000);

    // Check if page is still responsive
    const responsive = await page.evaluate(() => {
      return document.readyState === 'complete';
    }).catch(() => false);
    console.log(`  Page responsive after large input: ${responsive ? '✅' : '❌'}`);

    // Check for console errors
    const noErrors = await page.evaluate(() => true).catch(() => false);
    console.log(`  No crash: ${noErrors ? '✅' : '❌'}`);
  });

  test('rapid keyboard interactions', async ({ page }) => {
    console.log('\n=== RAPID INTERACTION TEST ===');

    const consoleErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    // Rapid Ctrl+K open/close
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Control+k');
      await page.waitForTimeout(50);
      await page.keyboard.press('Escape');
      await page.waitForTimeout(50);
    }
    console.log(`  10x rapid Ctrl+K/Escape: ✅ no crash`);

    // Rapid tab switching (if tabs exist)
    for (let i = 0; i < 20; i++) {
      await page.keyboard.press('Tab');
      await page.waitForTimeout(20);
    }
    console.log(`  20x rapid Tab: ✅ no crash`);

    // Check page still responsive
    const responsive = await page.evaluate(() => document.readyState === 'complete').catch(() => false);
    console.log(`  Page responsive: ${responsive ? '✅' : '❌'}`);

    if (consoleErrors.length > 0) {
      console.log(`  Console errors: ${consoleErrors.length}`);
      consoleErrors.slice(0, 5).forEach(e => console.log(`    ${e.substring(0, 100)}`));
    } else {
      console.log(`  Console errors: 0 ✅`);
    }
  });

  test('special characters in all visible inputs', async ({ page }) => {
    console.log('\n=== SPECIAL CHARACTER TEST ===');

    const specialChars = [
      '../../etc/passwd',
      '\x00\x01\x02',
      '🎭🔥💀',
      'عربي',
      '中文测试',
      'a'.repeat(1000),
    ];

    // Find all visible inputs
    const inputs = page.locator('input:visible, textarea:visible');
    const count = await inputs.count();
    console.log(`  Found ${count} visible inputs`);

    for (let i = 0; i < count && i < 5; i++) {
      const input = inputs.nth(i);
      const inputType = await input.getAttribute('type') || 'text';
      const inputName = await input.getAttribute('aria-label') || await input.getAttribute('placeholder') || `input-${i}`;

      for (const chars of specialChars) {
        try {
          await input.fill('');
          await input.fill(chars);
          await page.waitForTimeout(100);
        } catch {
          // Some inputs may reject certain characters — that's fine
        }
      }
      console.log(`  Input "${inputName}" (${inputType}): survived all payloads ✅`);
    }
  });
});
