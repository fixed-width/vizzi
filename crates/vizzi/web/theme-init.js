// Apply only a valid explicit override before styles paint; CSS owns System mode.
try {
  const theme=localStorage.getItem('vizzi.theme.v1');
  if(theme==='light' || theme==='dark')document.documentElement.dataset.theme=theme;
} catch {}
