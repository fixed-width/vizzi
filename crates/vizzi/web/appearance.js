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

export function mapPalette(canvas) {
  const styles=getComputedStyle(canvas),read=name=>styles.getPropertyValue(`--${name}`).trim();
  return {text:read('text'),muted:read('muted'),selected:read('accent-fill'),onSelected:read('on-accent'),file:read('map-file'),partial:read('map-partial'),unavailable:read('map-unavailable'),group:read('map-group'),border:read('border'),focus:read('focus')};
}
