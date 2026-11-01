export function followChanges({ session, createSource = url => new EventSource(url), schedule = setTimeout, cancel = clearTimeout }) {
  let current, timer, stopped = false, backoff = 100;
  function close(connection, error) {
    current = undefined;
    connection.source?.close();
    connection.consumer.close(error);
  }
  function retry(connection, error) {
    if (current !== connection) return;
    close(connection, error);
    timer = schedule(() => { timer = undefined; open(); }, backoff);
    backoff = Math.min(4000, backoff * 2);
  }
  function open() {
    if (stopped) return;
    const connection = current = { consumer: session() };
    try {
      const source = connection.source = createSource("/api/events");
      source.addEventListener("workspace", event => {
        if (current !== connection) return;
        try { if (connection.consumer.receive(JSON.parse(event.data))) backoff = 100; }
        catch (error) { retry(connection, error); }
      });
      source.addEventListener("error", () => retry(connection, new Error("Live updates disconnected")));
    } catch (error) { retry(connection, error); }
  }
  open();
  return {
    restart() { if (current) retry(current); },
    stop() {
      stopped = true;
      if (timer !== undefined) cancel(timer);
      if (current) close(current);
    },
  };
}
