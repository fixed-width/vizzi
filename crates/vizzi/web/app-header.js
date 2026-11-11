const views = [
 {id: 'calls', label: 'Call workspace', href: '/'},
 {id: 'review', label: 'Review changes', href: '/review'},
 {id: 'source', label: 'Source browser', href: '/explore'},
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
 settings.innerHTML = '<summary>Settings</summary><div><label>Theme<select id="theme" aria-label="Theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label></div>';
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
