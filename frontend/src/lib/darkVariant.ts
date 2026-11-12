/**
 * Which lighter dark theme the page uses (`dark-variants.css`): `?darkv=1`
 * or `?darkv=2` in the address picks one and the browser keeps it; `?darkv=0`
 * is the original. Variant 1 when nothing was picked. Runs before the first
 * paint, next to the theme script.
 */
export const DARK_VARIANT_SCRIPT = `(function(){try{var v=new URLSearchParams(window.location.search).get('darkv');if(v==='0'||v==='1'||v==='2'){window.localStorage.setItem('darkv',v);}else{v=window.localStorage.getItem('darkv');}if(v!=='0'&&v!=='2'){v='1';}document.documentElement.setAttribute('data-darkv',v);}catch(e){document.documentElement.setAttribute('data-darkv','1');}})();`;
