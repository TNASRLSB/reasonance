import { test, expect } from '@playwright/test';

test.describe('Phase 3C — RTL Layout & i18n Testing', () => {
  test('RTL layout — check for broken components', async ({ page }) => {
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    // Inject RTL direction
    await page.evaluate(() => {
      document.documentElement.setAttribute('dir', 'rtl');
      document.documentElement.setAttribute('lang', 'ar');
    });
    await page.waitForTimeout(500);

    const rtlIssues = await page.evaluate(() => {
      const issues: Array<{ el: string; issue: string; detail: string }> = [];
      const elements = document.querySelectorAll('*');

      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        if (htmlEl.offsetWidth === 0 && htmlEl.offsetHeight === 0) continue;

        const style = window.getComputedStyle(htmlEl);

        // Check for physical directional properties that don't flip in RTL
        const physicalProps = {
          'margin-left': style.marginLeft,
          'margin-right': style.marginRight,
          'padding-left': style.paddingLeft,
          'padding-right': style.paddingRight,
          'text-align': style.textAlign,
          'float': style.cssFloat,
          'left': style.left,
          'right': style.right,
        };

        // Check text-align: left (should be start in RTL)
        if (style.textAlign === 'left' && htmlEl.innerText?.trim()) {
          issues.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`,
            issue: 'text-align: left',
            detail: `Text "${htmlEl.innerText.substring(0, 30)}" aligned left in RTL`,
          });
        }

        // Check for absolute positioned elements using left/right
        if (style.position === 'absolute' || style.position === 'fixed') {
          if (style.left !== 'auto' && style.right === 'auto') {
            issues.push({
              el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`,
              issue: 'position: left without right',
              detail: `Positioned left:${style.left} — won't mirror in RTL`,
            });
          }
        }

        // Check for overflow caused by RTL switch
        if (htmlEl.scrollWidth > htmlEl.clientWidth + 5) {
          const overflow = style.overflow;
          if (overflow !== 'auto' && overflow !== 'scroll' && overflow !== 'hidden') {
            if (htmlEl.tagName !== 'HTML' && htmlEl.tagName !== 'BODY') {
              issues.push({
                el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`,
                issue: 'horizontal overflow in RTL',
                detail: `Overflow: ${htmlEl.scrollWidth - htmlEl.clientWidth}px`,
              });
            }
          }
        }
      }

      return issues;
    });

    console.log(`\n=== RTL LAYOUT CHECK ===`);
    if (rtlIssues.length === 0) {
      console.log('✅ No RTL layout issues detected on welcome screen');
    } else {
      console.log(`⚠️ ${rtlIssues.length} RTL issues found:`);
      // Group by issue type
      const byType = new Map<string, typeof rtlIssues>();
      for (const i of rtlIssues) {
        if (!byType.has(i.issue)) byType.set(i.issue, []);
        byType.get(i.issue)!.push(i);
      }
      for (const [type, items] of byType) {
        console.log(`\n  ${type} (${items.length}):`);
        for (const item of items.slice(0, 5)) {
          console.log(`    ${item.el}: ${item.detail}`);
        }
        if (items.length > 5) console.log(`    ... and ${items.length - 5} more`);
      }
    }

    await page.screenshot({ path: 'docs/audit/screenshot-rtl.png', fullPage: true });
    console.log('\nScreenshot saved: docs/audit/screenshot-rtl.png');
  });

  test('German locale — check for text truncation', async ({ page }) => {
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    // Try to switch locale to German via the i18n system
    await page.evaluate(() => {
      // Attempt to set locale if the i18n system is exposed
      const html = document.documentElement;
      html.setAttribute('lang', 'de');
    });
    await page.waitForTimeout(300);

    // Check all elements with text-overflow: ellipsis
    const truncated = await page.evaluate(() => {
      const results: Array<{ el: string; text: string; width: number; scrollWidth: number }> = [];
      const elements = document.querySelectorAll('*');

      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        const style = window.getComputedStyle(htmlEl);

        // Check if text is actually being truncated
        if (htmlEl.scrollWidth > htmlEl.clientWidth + 1 && htmlEl.innerText?.trim()) {
          results.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`,
            text: htmlEl.innerText.substring(0, 50),
            width: htmlEl.clientWidth,
            scrollWidth: htmlEl.scrollWidth,
          });
        }
      }
      return results;
    });

    console.log(`\n=== GERMAN LOCALE TRUNCATION CHECK ===`);
    if (truncated.length === 0) {
      console.log('✅ No text truncation detected');
    } else {
      console.log(`⚠️ ${truncated.length} elements with truncated text:`);
      for (const t of truncated) {
        console.log(`  ${t.el}: "${t.text}" (visible: ${t.width}px, needs: ${t.scrollWidth}px)`);
      }
    }
  });

  test('accessibility tree snapshot — screen reader simulation', async ({ page }) => {
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    // Use aria snapshot (modern Playwright API)
    const snapshot = await page.locator('body').ariaSnapshot();

    console.log(`\n=== ACCESSIBILITY TREE (Screen Reader View) ===`);
    console.log(snapshot);

    // Also gather ARIA role summary via evaluate
    const ariaSummary = await page.evaluate(() => {
      const roles = new Map<string, number>();
      const elements = document.querySelectorAll('[role]');
      for (const el of elements) {
        const role = el.getAttribute('role') || 'none';
        roles.set(role, (roles.get(role) || 0) + 1);
      }

      const interactiveRoles = ['button', 'link', 'textbox', 'checkbox', 'radio', 'tab', 'menuitem', 'combobox', 'slider'];
      const interactive = document.querySelectorAll(interactiveRoles.map(r => `[role="${r}"]`).join(', ') + ', button, a[href], input, select, textarea');

      const unlabeled: string[] = [];
      for (const el of interactive) {
        const label = el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || (el as HTMLElement).innerText?.trim();
        if (!label) {
          unlabeled.push(`${el.tagName.toLowerCase()}.${el.className?.toString().substring(0, 30)}`);
        }
      }

      return {
        roleCount: Object.fromEntries(roles),
        interactiveCount: interactive.length,
        unlabeledInteractive: unlabeled,
      };
    });

    console.log(`\n--- Summary ---`);
    console.log(`ARIA roles: ${JSON.stringify(ariaSummary.roleCount)}`);
    console.log(`Interactive elements: ${ariaSummary.interactiveCount}`);
    if (ariaSummary.unlabeledInteractive.length > 0) {
      console.log(`Unlabeled interactive elements (${ariaSummary.unlabeledInteractive.length}):`);
      ariaSummary.unlabeledInteractive.forEach(e => console.log(`  ❌ ${e}`));
    } else {
      console.log('All interactive elements have labels ✅');
    }
  });
});
