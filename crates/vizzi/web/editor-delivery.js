import { createDeliveryPage, deliveryId, deliveryAction } from '/workspace-delivery.mjs';
export function installWorkspaceDelivery({
  mode,
  ready,
  prepare,
  failed = () => {},
  changed = () => {},
  window = globalThis.window,
  document = globalThis.document
}) {
  const parameters = new URLSearchParams(window.location.hash.replace(/^#/, ''));
  let transition = parameters.get('vizzi-delivery');
  if (transition) {
    parameters.delete('vizzi-delivery');
    const hash = parameters.toString();
    window.history.replaceState(window.history.state, '', window.location.pathname + window.location.search + (hash ? '#' + hash : ''));
  }
  if (window.parent === window) transition = null;
  const identity = deliveryId();
  let port,
    closed = false;
  const controller = createDeliveryPage({
    mode,
    document: identity,
    transition,
    ready,
    prepare,
    failed,
    changed,
    send: message => port?.postMessage(message),
    navigate: (path, ticket) => window.location.assign(path + '#vizzi-delivery=' + ticket)
  });
  const hello = () => {
    if (!closed && window.parent !== window) window.parent.postMessage({
      protocol: 'vizzi-delivery-connect',
      version: 1,
      type: 'hello',
      document: identity,
      mode,
      transition
    }, '*');
  };
  function attach(event) {
    const data = event.data;
    if (event.source !== window.parent || data?.protocol !== 'vizzi-delivery-connect' || data.version !== 1) return;
    if (data.type === 'probe') {
      hello();
      return;
    }
    if (data.type !== 'attach' || data.document !== identity || typeof data.challenge !== 'string' || !event.ports?.[0]) return;
    port?.close();
    port = event.ports[0];
    port.onmessage = message => controller.receive(message.data);
    port.start();
    port.postMessage({
      protocol: 'vizzi-delivery-connect',
      version: 1,
      type: 'attached',
      document: identity,
      challenge: data.challenge,
      mode,
      transition
    });
    controller.connect();
  }
  function manual() {
    if (controller.manual()) window.stop();
  }
  function link(event) {
    if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
    const target = event.target?.closest?.('a[href]');
    if (!target || target.target && target.target !== '_self' || target.hasAttribute('download')) return;
    const address = new URL(target.href, window.location.href);
    if (address.origin === window.location.origin && ['/', '/workspace', '/explore', '/review', '/investigate'].includes(address.pathname)) manual();
  }
  function legacy(hash) {
    const values = new URLSearchParams(hash.replace(/^#/, ''));
    let action, keys;
    if (mode === 'call' && values.has('saved-bubble')) {
      action = {kind:'bubble', snapshot:values.get('snapshot'), file:values.get('file'), version:values.get('version'), line:Number(values.get('line')), declaration:values.get('saved-bubble')};
      keys = ['saved-bubble', 'snapshot', 'file', 'version', 'line'];
    } else if (mode === 'call' && values.has('source-function')) {
      action = {kind:'function', snapshot:values.get('snapshot'), file:values.get('file'), symbol:values.get('source-function')};
      keys = ['source-function', 'snapshot', 'file'];
    } else if (mode === 'call' && values.has('source-call')) {
      action = {kind:'call', snapshot:values.get('snapshot'), file:values.get('file'), call:values.get('source-call')};
      keys = ['source-call', 'snapshot', 'file'];
    } else if (mode === 'call' && values.has('editor-call')) {
      action = {
        kind: 'call',
        snapshot: values.get('snapshot'),
        call: values.get('editor-call')
      };
      keys = ['editor-call', 'snapshot'];
    }
    if (mode === 'file' && values.has('editor-file')) {
      action = {
        kind: 'file',
        snapshot: values.get('snapshot'),
        file: values.get('editor-file'),
        version: values.get('version'),
        line: Number(values.get('line'))
      };
      keys = ['editor-file', 'snapshot', 'version', 'line'];
    }
    if (!action) return;
    try {
      action = deliveryAction(action);
    } catch (error) {
      failed(error);
      return;
    }
    if (window.location.hash === hash) {
      for (const key of keys) values.delete(key);
      const remaining = values.toString();
      window.history.replaceState(window.history.state, '', window.location.pathname + window.location.search + (remaining ? '#' + remaining : ''));
    }
    void controller.submit(action, {
      replaceHistory: true
    }).catch(() => {});
  }
  const hashChanged = event => {
    const hash = new URL(event.newURL).hash;
    if (window.location.hash === hash) legacy(hash);
  };
  const leaving = () => controller.depart();
  window.addEventListener('message', attach);
  document.addEventListener('click', link, true);
  window.addEventListener('hashchange', hashChanged);
  window.addEventListener('pagehide', leaving);
  hello();
  legacy(window.location.hash);
  return {
    ...controller,
    manual,
    dispose() {
      closed = true;
      controller.dispose();
      port?.close();
      window.removeEventListener('message', attach);
      document.removeEventListener('click', link, true);
      window.removeEventListener('pagehide', leaving);
      window.removeEventListener('hashchange', hashChanged);
    }
  };
}
