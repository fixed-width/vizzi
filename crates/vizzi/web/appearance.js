export function installAppearance(select,{root=document.documentElement,storage=()=>localStorage,media=matchMedia('(prefers-color-scheme: dark)'),onChange=()=>{}}={}) {
  let choice='system';
  function apply(value){
    choice=['light','dark'].includes(value)?value:'system';select.value=choice;
    if(choice==='system')delete root.dataset.theme;else root.dataset.theme=choice;
    onChange(choice==='system'?(media.matches?'dark':'light'):choice);
  }
  try {choice=storage().getItem('vizzi.theme.v1');}catch{}
  apply(choice);
  select.addEventListener('change',()=>{apply(select.value);try{storage().setItem('vizzi.theme.v1',choice);}catch{}});
  media.addEventListener('change',()=>{if(choice==='system')onChange(media.matches?'dark':'light');});
  root.ownerDocument.defaultView.addEventListener('storage',event=>{if(event.key==='vizzi.theme.v1')apply(event.newValue);});
}

export const mapFileTypes = {
  c: 'C',
  cpp: 'C++',
  cuda: 'CUDA',
  java: 'Java',
  javascript: 'JavaScript',
  python: 'Python',
  ruby: 'Ruby',
  rust: 'Rust',
  typescript: 'TypeScript',
  documentation: 'Documentation',
  json: 'JSON',
  toml: 'TOML',
  metadata: 'Project metadata',
  other: 'Other files',
};

export function mapPalette(canvas) {
  const styles = getComputedStyle(canvas);
  const read = name => styles.getPropertyValue(`--${name}`);
  const types = Object.fromEntries(Object.keys(mapFileTypes).map(type =>
    [type, read(`map-${type}-${styles.colorScheme}`)]));
  return {
    types,
    text: read('text'),
    partial: read('map-partial'),
    unavailable: read('muted'),
    group: read('map-group'),
    border: read('border'),
    focus: read('focus'),
  };
}
