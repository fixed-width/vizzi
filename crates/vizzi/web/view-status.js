export function workspaceLabel(status) {
 return status.error?'Refresh failed; showing captured source':'Source ready';
}
export function reviewLabel(status) {
 if(status.error)return 'Comparison failed';
 if(status.active)return 'Comparison ready';
 return status.pending?'Capturing comparison…':'Choose a base revision to begin';
}
