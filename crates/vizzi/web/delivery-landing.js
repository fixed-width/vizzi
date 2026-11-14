import {installWorkspaceDelivery} from '/editor-delivery.js';
installWorkspaceDelivery({mode:'landing',ready:()=>true,prepare:async()=>({commit(){}})});
