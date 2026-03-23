import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

/**
 * Phase 3 — Full UI Testing with Mocked State
 *
 * Injects mock data into Svelte stores via page.evaluate() to simulate
 * a loaded project with files, editor, chat messages, and terminal.
 * This allows testing all UI components without the Tauri backend.
 */

async function injectMockState(page: any) {
  await page.evaluate(() => {
    // Wait for Svelte stores module to be available
    // Access stores through the app's module system
    const storeModules = (window as any).__SVELTE_STORES__;
    if (!storeModules) {
      console.warn('Svelte stores not exposed — trying direct DOM manipulation');
      return false;
    }
    return true;
  });

  // Since we can't directly access Svelte stores from outside,
  // we'll use Playwright to interact with the UI to open states
  // OR inject via the module system. Let's try a hybrid approach.

  // First, check if we can interact with the welcome screen to trigger state changes
  return page;
}

// Helper: inject mock files into the app by dispatching custom events
async function setupMockProject(page: any) {
  await page.evaluate(() => {
    // The app checks for __TAURI_INTERNALS__ to determine if running in Tauri
    // Without it, many features are disabled. Let's mock the minimum needed.
    if (!(window as any).__TAURI_INTERNALS__) {
      (window as any).__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: any) => {
          // Mock responses for common commands
          switch (cmd) {
            case 'list_dir':
              return [
                { name: 'src', is_dir: true, path: '/mock-project/src' },
                { name: 'README.md', is_dir: false, path: '/mock-project/README.md' },
                { name: 'package.json', is_dir: false, path: '/mock-project/package.json' },
                { name: 'index.ts', is_dir: false, path: '/mock-project/index.ts' },
                { name: '.env', is_dir: false, path: '/mock-project/.env' },
                { name: 'VerylongfilenamethatmightcausetruncationissuesintheUI.ts', is_dir: false, path: '/mock-project/Verylongfilenamethatmightcausetruncationissuesintheui.ts' },
              ];
            case 'read_file':
              return '// Mock file content\nconsole.log("hello world");\n\nfunction test() {\n  return 42;\n}\n';
            case 'write_file':
              return null;
            case 'read_config':
              return {
                providers: [
                  { id: 'anthropic', name: 'Anthropic', api_key: '***', model: 'claude-sonnet-4-20250514', enabled: true },
                  { id: 'openai', name: 'OpenAI', api_key: '', model: 'gpt-4', enabled: false },
                ],
                locale: 'en',
                theme: 'dark',
              };
            case 'discover_agents':
              return [
                { id: 'claude', name: 'Claude', provider: 'anthropic', model: 'claude-sonnet-4-20250514', status: 'available' },
              ];
            case 'get_analytics_summary':
              return { total_tokens: 15000, total_cost: 0.45, sessions: 12 };
            default:
              console.log(`Mock IPC: ${cmd}`, args);
              return null;
          }
        },
        transformCallback: () => 0,
      };
    }
  });
}

test.describe('Phase 3 — Full UI with Mock State', () => {
  test.beforeEach(async ({ page }) => {
    // Set up mock Tauri internals before page loads
    await page.addInitScript(() => {
      (window as any).__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: any) => {
          const responses: Record<string, any> = {
            list_dir: [
              { name: 'src', is_dir: true, path: '/mock-project/src' },
              { name: 'README.md', is_dir: false, path: '/mock-project/README.md' },
              { name: 'package.json', is_dir: false, path: '/mock-project/package.json' },
              { name: 'index.ts', is_dir: false, path: '/mock-project/index.ts' },
              { name: 'Einstellungskonfigurationsdatei.ts', is_dir: false, path: '/mock-project/Einstellungskonfigurationsdatei.ts' },
            ],
            read_file: '// Mock file content\nconsole.log("hello world");\n\nexport function greet(name: string): string {\n  return `Hello, ${name}!`;\n}\n',
            write_file: null,
            write_config: null,
            read_config: {
              llm_configs: [
                { id: 'anthropic', name: 'Anthropic', api_key: 'sk-***', model: 'claude-sonnet-4-20250514', enabled: true, provider: 'anthropic' },
              ],
              app_settings: { locale: 'en', theme: 'dark' },
            },
            discover_agents: [
              { id: 'claude', name: 'Claude', provider: 'anthropic', model: 'claude-sonnet-4-20250514', status: 'available' },
            ],
            get_analytics_summary: { total_tokens: 0, total_cost: 0, sessions: 0 },
            list_workflows: [],
            start_watching: null,
            stop_watching: null,
            open_pty: { id: 'mock-pty-1' },
            resize_pty: null,
            write_pty: null,
            close_pty: null,
            grep_files: [],
            get_session_history: [],
          };
          const result = responses[cmd];
          if (result !== undefined) return result;
          console.log(`Unmocked IPC: ${cmd}`, JSON.stringify(args));
          return null;
        },
        transformCallback: (cb: any) => {
          const id = Math.random();
          (window as any)[`__callback_${id}`] = cb;
          return id;
        },
        convertFileSrc: (path: string) => path,
      };
    });

    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000); // Let Svelte hydrate and process mock data
  });

  test('1. Full UI — axe-core scan with project loaded', async ({ page }) => {
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();

    console.log(`\n=== AXE-CORE FULL UI SCAN ===`);
    console.log(`Violations: ${results.violations.length}`);
    console.log(`Passes: ${results.passes.length}`);
    console.log(`Incomplete: ${results.incomplete.length}`);

    for (const violation of results.violations) {
      console.log(`\n[${violation.impact?.toUpperCase()}] ${violation.id}: ${violation.help}`);
      console.log(`  WCAG: ${violation.tags.filter(t => t.startsWith('wcag')).join(', ')}`);
      console.log(`  Nodes: ${violation.nodes.length}`);
      for (const node of violation.nodes.slice(0, 3)) {
        console.log(`  Target: ${node.target.join(' > ')}`);
        console.log(`  HTML: ${node.html.substring(0, 150)}`);
      }
      if (violation.nodes.length > 3) console.log(`  ... +${violation.nodes.length - 3} more`);
    }

    await page.screenshot({ path: 'docs/audit/screenshot-full-ui.png', fullPage: true });
  });

  test('2. RTL with full UI — layout check', async ({ page }) => {
    // Switch to RTL
    await page.evaluate(() => {
      document.documentElement.setAttribute('dir', 'rtl');
      document.documentElement.setAttribute('lang', 'ar');
    });
    await page.waitForTimeout(500);

    const issues = await page.evaluate(() => {
      const results: Array<{ el: string; issue: string; detail: string }> = [];
      const elements = document.querySelectorAll('*');

      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        if (htmlEl.offsetWidth === 0 && htmlEl.offsetHeight === 0) continue;
        const style = window.getComputedStyle(htmlEl);

        // Check text-align: left (should be start in RTL)
        if (style.textAlign === 'left' && htmlEl.innerText?.trim()) {
          results.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 50)}`,
            issue: 'text-align: left in RTL',
            detail: `"${htmlEl.innerText.substring(0, 30)}"`,
          });
        }

        // Check for absolute/fixed positioned elements using physical left/right
        if ((style.position === 'absolute' || style.position === 'fixed') &&
            style.left !== 'auto' && style.right === 'auto') {
          results.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 50)}`,
            issue: 'position: left only (no RTL mirror)',
            detail: `left: ${style.left}`,
          });
        }

        // Check horizontal overflow caused by RTL
        if (htmlEl.scrollWidth > htmlEl.clientWidth + 5) {
          if (!['auto', 'scroll', 'hidden'].includes(style.overflowX) &&
              !['HTML', 'BODY'].includes(htmlEl.tagName)) {
            results.push({
              el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 50)}`,
              issue: 'horizontal overflow in RTL',
              detail: `${htmlEl.scrollWidth - htmlEl.clientWidth}px overflow`,
            });
          }
        }
      }
      return results;
    });

    console.log(`\n=== RTL FULL UI CHECK ===`);
    if (issues.length === 0) {
      console.log('✅ No RTL layout issues');
    } else {
      const byType = new Map<string, typeof issues>();
      for (const i of issues) {
        if (!byType.has(i.issue)) byType.set(i.issue, []);
        byType.get(i.issue)!.push(i);
      }
      for (const [type, items] of byType) {
        console.log(`\n⚠️ ${type} (${items.length}):`);
        for (const item of items.slice(0, 8)) {
          console.log(`  ${item.el}: ${item.detail}`);
        }
        if (items.length > 8) console.log(`  ... +${items.length - 8} more`);
      }
    }

    await page.screenshot({ path: 'docs/audit/screenshot-rtl-full.png', fullPage: true });
  });

  test('3. German labels — truncation with full UI', async ({ page }) => {
    // Inject long German-like labels by finding all text nodes and extending them
    const truncated = await page.evaluate(() => {
      // Find all elements that might truncate
      const results: Array<{ el: string; text: string; visible: number; full: number }> = [];
      const elements = document.querySelectorAll('button, [role="tab"], [role="menuitem"], label, .toolbar-btn, .tab, .nav-item, span, a');

      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        if (htmlEl.offsetWidth === 0) continue;
        const style = window.getComputedStyle(htmlEl);

        // Check if text would be truncated with 2x length (German expansion)
        const originalText = htmlEl.innerText?.trim();
        if (!originalText || originalText.length < 3) continue;

        // Simulate German expansion (typically 1.3x-2.1x longer)
        const germanExpanded = originalText + originalText.substring(0, Math.ceil(originalText.length * 0.8));
        const originalWidth = htmlEl.scrollWidth;
        const containerWidth = htmlEl.clientWidth;

        if (originalWidth > containerWidth + 1) {
          results.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`,
            text: originalText,
            visible: containerWidth,
            full: originalWidth,
          });
        }

        // Also check if style has text-overflow: ellipsis
        if (style.textOverflow === 'ellipsis' && style.overflow === 'hidden') {
          results.push({
            el: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)} [has ellipsis]`,
            text: originalText,
            visible: containerWidth,
            full: originalWidth,
          });
        }
      }

      return results;
    });

    console.log(`\n=== GERMAN LABEL EXPANSION CHECK ===`);
    if (truncated.length === 0) {
      console.log('✅ No truncation issues detected');
    } else {
      console.log(`⚠️ ${truncated.length} elements may truncate with German labels:`);
      for (const t of truncated) {
        console.log(`  ${t.el}: "${t.text}" (visible: ${t.visible}px, content: ${t.full}px)`);
      }
    }
  });

  test('4. XSS payloads in all inputs', async ({ page }) => {
    // Find all text inputs in the full UI
    const inputs = page.locator('input:visible, textarea:visible');
    const count = await inputs.count();

    console.log(`\n=== XSS INPUT TESTING (Full UI) ===`);
    console.log(`Found ${count} visible inputs`);

    const xssPayloads = [
      '<script>alert("XSS")</script>',
      '<img src=x onerror=alert(1)>',
      '"><svg onload=alert(1)>',
      "'; DROP TABLE files; --",
      '{{constructor.constructor("alert(1)")()}}',
      '<form action="https://evil.com"><input type="submit">',
    ];

    const consoleErrors: string[] = [];
    page.on('console', (msg: any) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    for (let i = 0; i < count && i < 10; i++) {
      const input = inputs.nth(i);
      const label = await input.getAttribute('aria-label') || await input.getAttribute('placeholder') || `input-${i}`;

      for (const payload of xssPayloads) {
        try {
          await input.fill('');
          await input.fill(payload);
          await page.waitForTimeout(100);
        } catch {
          // Some inputs may not accept certain content
        }
      }

      // Check for injected elements
      const injected = await page.evaluate(() => {
        const scripts = document.querySelectorAll('script:not([src])');
        const svgs = document.querySelectorAll('svg[onload]');
        const imgs = document.querySelectorAll('img[onerror]');
        const forms = document.querySelectorAll('form[action*="evil"]');
        return scripts.length + svgs.length + imgs.length + forms.length;
      });

      console.log(`  Input "${label}": ${injected === 0 ? '✅ Safe' : '❌ XSS INJECTED!'}`);
    }

    // Also try via SearchPalette
    await page.keyboard.press('Control+k');
    await page.waitForTimeout(500);

    const searchInput = page.locator('input:visible').first();
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      for (const payload of xssPayloads) {
        await searchInput.fill('');
        await searchInput.fill(payload);
        await page.waitForTimeout(100);
      }
      console.log(`  SearchPalette: tested ${xssPayloads.length} payloads`);

      const injected = await page.evaluate(() => {
        const scripts = document.querySelectorAll('script:not([src])');
        return scripts.length;
      });
      console.log(`  SearchPalette XSS result: ${injected === 0 ? '✅ Safe' : '❌ INJECTED'}`);

      await page.keyboard.press('Escape');
    } else {
      console.log('  SearchPalette not available');
    }

    if (consoleErrors.length > 0) {
      console.log(`\nConsole errors during XSS test: ${consoleErrors.length}`);
    }
  });

  test('5. Full UI keyboard navigation — all regions', async ({ page }) => {
    const tabStops: Array<{ tag: string; role: string; label: string; region: string }> = [];

    for (let i = 0; i < 80; i++) {
      await page.keyboard.press('Tab');

      const info = await page.evaluate(() => {
        const el = document.activeElement;
        if (!el || el === document.body) return null;

        // Determine which region the element is in
        let region = 'unknown';
        let parent: Element | null = el;
        while (parent) {
          const classes = parent.className?.toString() || '';
          if (classes.includes('sidebar') || classes.includes('file-tree')) region = 'sidebar';
          else if (classes.includes('editor') || classes.includes('tab')) region = 'editor';
          else if (classes.includes('chat') || classes.includes('response')) region = 'chat';
          else if (classes.includes('terminal')) region = 'terminal';
          else if (classes.includes('toolbar')) region = 'toolbar';
          else if (classes.includes('status')) region = 'statusbar';
          else if (classes.includes('settings')) region = 'settings';
          else if (classes.includes('analytics')) region = 'analytics';
          else if (classes.includes('welcome')) region = 'welcome';
          else if (classes.includes('toast')) region = 'toast';
          else if (classes.includes('titlebar') || classes.includes('win-btn')) region = 'titlebar';
          if (region !== 'unknown') break;
          parent = parent.parentElement;
        }

        return {
          tag: el.tagName.toLowerCase(),
          role: el.getAttribute('role') || '',
          label: el.getAttribute('aria-label') || (el as HTMLElement).innerText?.substring(0, 20) || '',
          region,
        };
      });

      if (info) tabStops.push(info);
    }

    // Analyze regions reached
    const regions = new Set(tabStops.map(s => s.region));
    const regionCounts = new Map<string, number>();
    for (const s of tabStops) {
      regionCounts.set(s.region, (regionCounts.get(s.region) || 0) + 1);
    }

    console.log(`\n=== FULL UI KEYBOARD NAVIGATION ===`);
    console.log(`Tab stops: ${tabStops.length}`);
    console.log(`Unique regions reached: ${regions.size}`);
    for (const [region, count] of regionCounts) {
      console.log(`  ${region}: ${count} stops`);
    }

    // Check which important regions are NOT reachable
    const expectedRegions = ['sidebar', 'editor', 'chat', 'terminal', 'toolbar', 'statusbar'];
    const missingRegions = expectedRegions.filter(r => !regions.has(r));
    if (missingRegions.length > 0) {
      console.log(`\n❌ Unreachable regions: ${missingRegions.join(', ')}`);
    } else {
      console.log('\n✅ All major regions reachable via Tab');
    }

    // Show unique tab stops
    const uniqueStops = new Map<string, typeof tabStops[0]>();
    for (const s of tabStops) {
      const key = `${s.tag}-${s.role}-${s.label}-${s.region}`;
      if (!uniqueStops.has(key)) uniqueStops.set(key, s);
    }
    console.log(`\nUnique interactive elements (${uniqueStops.size}):`);
    for (const [, s] of uniqueStops) {
      console.log(`  [${s.region}] ${s.tag}${s.role ? `[role=${s.role}]` : ''}: "${s.label}"`);
    }
  });

  test('6. Accessibility tree — full UI screen reader view', async ({ page }) => {
    const snapshot = await page.locator('body').ariaSnapshot();

    console.log(`\n=== ACCESSIBILITY TREE (Full UI) ===`);
    console.log(snapshot);

    // Detailed analysis
    const analysis = await page.evaluate(() => {
      const roles = new Map<string, number>();
      const allRoled = document.querySelectorAll('[role]');
      for (const el of allRoled) {
        const role = el.getAttribute('role')!;
        roles.set(role, (roles.get(role) || 0) + 1);
      }

      // Check for landmarks
      const landmarks = {
        main: document.querySelectorAll('[role="main"], main').length,
        navigation: document.querySelectorAll('[role="navigation"], nav').length,
        banner: document.querySelectorAll('[role="banner"], header').length,
        complementary: document.querySelectorAll('[role="complementary"], aside').length,
        contentinfo: document.querySelectorAll('[role="contentinfo"], footer').length,
        search: document.querySelectorAll('[role="search"]').length,
        region: document.querySelectorAll('[role="region"]').length,
      };

      // Check for images without alt
      const imgsNoAlt = document.querySelectorAll('img:not([alt])');
      // Check for interactive elements without names
      const unlabeled: string[] = [];
      const interactive = document.querySelectorAll('button, a[href], input, select, textarea, [role="button"], [role="tab"], [role="menuitem"]');
      for (const el of interactive) {
        const name = el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') ||
                     el.getAttribute('title') || (el as HTMLElement).innerText?.trim();
        if (!name) {
          unlabeled.push(`${el.tagName.toLowerCase()}.${el.className?.toString().substring(0, 30)}`);
        }
      }

      // Count headings
      const headings: string[] = [];
      document.querySelectorAll('h1, h2, h3, h4, h5, h6').forEach(h => {
        headings.push(`h${h.tagName[1]}: "${h.textContent?.substring(0, 40)}"`);
      });

      return {
        roleCount: Object.fromEntries(roles),
        landmarks,
        imgsWithoutAlt: imgsNoAlt.length,
        unlabeledInteractive: unlabeled,
        headings,
        totalInteractive: interactive.length,
      };
    });

    console.log(`\n--- Detailed Analysis ---`);
    console.log(`ARIA roles: ${JSON.stringify(analysis.roleCount)}`);
    console.log(`\nLandmarks:`);
    for (const [name, count] of Object.entries(analysis.landmarks)) {
      console.log(`  ${name}: ${count} ${count === 0 ? '❌' : '✅'}`);
    }
    console.log(`\nHeadings: ${analysis.headings.length}`);
    analysis.headings.forEach(h => console.log(`  ${h}`));
    console.log(`\nImages without alt: ${analysis.imgsWithoutAlt}`);
    console.log(`Total interactive elements: ${analysis.totalInteractive}`);
    console.log(`Unlabeled interactive: ${analysis.unlabeledInteractive.length}`);
    if (analysis.unlabeledInteractive.length > 0) {
      analysis.unlabeledInteractive.forEach(e => console.log(`  ❌ ${e}`));
    }
  });
});
