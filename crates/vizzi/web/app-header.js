const views = [
 {id: 'source', label: 'Source browser', href: '/'},
 {id: 'calls', label: 'Call workspace', href: '/workspace'},
 {id: 'review', label: 'Review changes', href: '/review'},
 {id: 'locations', label: 'Saved locations', href: '/investigate#saved'},
];

// Pages supply their existing controls; common navigation and menu behaviour live here.
export function installAppHeader(root) {
 const current = views.find(view => view.id === root.dataset.appView);
 if (!current) throw new Error('Unknown header view');
 const document = root.ownerDocument;
 const controls = root.querySelector('[data-header-controls]');
 if (!controls) throw new Error('Header controls are missing');
 const extra = root.querySelector('template[data-header-settings]');
 const bar = document.createElement('div');
 bar.className = 'app-bar';
 const brand = document.createElement('a');
 brand.className = 'brand';
 brand.href = '/';
 brand.textContent = 'vizzi';
 brand.setAttribute('aria-label', 'Vizzi home');
 const navigation = document.createElement('nav');
 navigation.className = 'view-navigation';
 navigation.setAttribute('aria-label', 'View modes');
 for (const view of views) {
  const link = document.createElement('a');
  link.href = view.href;
  link.textContent = view.label;
  if (view.id === current.id) link.setAttribute('aria-current', 'page');
  navigation.append(link);
 }
 const settings = document.createElement('details');
 settings.id = 'workspace-settings';
 settings.className = 'app-menu';
 settings.innerHTML = `<summary aria-label="Settings" title="Settings">
   <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" aria-hidden="true" focusable="false">
    <path d="M10.25 2h3.5l.5 2.75 1.25.5 2.3-1.55 2.5 2.5-1.55 2.3.5 1.25 2.75.5v3.5l-2.75.5-.5 1.25 1.55 2.3-2.5 2.5-2.3-1.55-1.25.5-.5 2.75h-3.5l-.5-2.75-1.25-.5-2.3 1.55-2.5-2.5 1.55-2.3-.5-1.25-2.75-.5v-3.5l2.75-.5.5-1.25-1.55-2.3 2.5-2.5 2.3 1.55 1.25-.5z"/>
    <circle cx="12" cy="12" r="3.25"/>
   </svg>
  </summary>
  <div><label>Theme<select id="theme" aria-label="Theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label></div>`;
 if (extra) settings.lastElementChild.append(extra.content);
 bar.append(brand, navigation, settings);
 controls.classList.add('mode-toolbar');
 controls.setAttribute('role', 'group');
 controls.setAttribute('aria-label', current.label + ' controls');
 root.className = 'app-header';
 root.replaceChildren(bar, controls);

 const menus = [...root.querySelectorAll('.app-menu')];
 for (const menu of menus) menu.addEventListener('toggle', () => {
  if (menu.open) for (const other of menus) if (other !== menu) other.open = false;
 });
 document.addEventListener('keydown', event => {
  if (event.key !== 'Escape') return;
  for (const menu of menus) if (menu.open) {
   menu.open = false;
   menu.querySelector('summary').focus();
  }
 });
 document.addEventListener('pointerdown', event => {
  for (const menu of menus) if (!menu.contains(event.target)) menu.open = false;
 });
}
