// Regenerate negotiated Markdown from the public HTML whenever website copy changes.
import fs from 'node:fs';
import path from 'node:path';
const root = process.argv[2] || process.cwd();
function decode(text) { return text.replace(/&#(\d+);/g, (_, n) => String.fromCodePoint(+n)).replace(/&#x([0-9a-f]+);/gi, (_, n) => String.fromCodePoint(parseInt(n, 16))).replace(/&(?:amp|lt|gt|quot|apos|nbsp|mdash|ndash|rsquo|lsquo|rarr);/g, value => ({'&amp;':'&','&lt;':'<','&gt;':'>','&quot;':'"','&apos;':"'",'&nbsp;':' ','&mdash;':'—','&ndash;':'–','&rsquo;':'’','&lsquo;':'‘','&rarr;':'→'}[value])); }
function markdown(html) {
 let text = html.match(/<main\b[^>]*>([\s\S]*?)<\/main>/i)?.[1] || html;
 return decode(text.replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi,'').replace(/<a\b[^>]*href="([^"]*)"[^>]*>([\s\S]*?)<\/a>/gi,(_,href,label)=>`[${label.replace(/<[^>]*>/g,'').trim()}](${new URL(decode(href),'https://thumble.io').href})`).replace(/<h([1-6])\b[^>]*>/gi,(_,level)=>'\n\n'+'#'.repeat(+level)+' ').replace(/<\/h[1-6]>/gi,'\n\n').replace(/<(?:p|section|article|details|summary|div|ol|ul|pre)\b[^>]*>/gi,'\n\n').replace(/<li\b[^>]*>/gi,'\n- ').replace(/<br\s*\/?>/gi,'\n').replace(/<[^>]+>/g,'')).replace(/[ \t]+/g,' ').replace(/\n +/g,'\n').replace(/\n{3,}/g,'\n\n').trim()+'\n';
}
const pages = {};
for (const file of fs.readdirSync(root).filter(name=>name.endsWith('.html') && name!=='404.html')) {
 const route = file==='index.html' ? '/' : '/'+file.slice(0,-5);
 pages[route]=markdown(fs.readFileSync(path.join(root,file),'utf8'));
}
fs.writeFileSync(path.join(root,'functions/_markdown.js'), 'export const documents = '+JSON.stringify(pages,null,2)+';\n');
