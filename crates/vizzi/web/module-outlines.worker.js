import {moduleContours} from './module-contours.js';
self.onmessage=event=>{const {ticket,views}=event.data;try{self.postMessage({ticket,paths:moduleContours(views)});}catch{self.postMessage({ticket,paths:[]});}};
