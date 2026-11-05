// A bounded message row: one current identity and one dismissal.
export function createMessage(root,{label='Dismiss message',content=root.querySelector('[data-message-text]')}={}) {
  const document=root.ownerDocument,initial=content?.textContent ?? root.textContent,wasHidden=root.hidden;
  if(!content){content=document.createElement('span');content.dataset.messageText='';root.replaceChildren(content);}
  const close=document.createElement('button');close.type='button';close.className='message-close';close.setAttribute('aria-label',label);close.title=label;root.append(close);root.classList.add('message-row');
  let current=JSON.stringify([initial,initial]),dismissed;content.textContent=initial;root.hidden=wasHidden || !initial;
  close.addEventListener('click',()=>{dismissed=current;root.hidden=true;});
  return {
    show(text,key=text){
      if(text==null || text===''){current=undefined;root.hidden=true;return;}
      const identity=JSON.stringify([key,text]);
      if(identity!==current){current=identity;dismissed=undefined;}
      content.textContent=text;root.hidden=current===dismissed;
    },
  };
}
