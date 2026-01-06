const fs = require('fs');
const path = '/home/mur/workspace/novelAi/novelAI_offecial_folder/chunks/6043-315495d0bee35961.js';

try {
  const content = fs.readFileSync(path, 'utf8');
  console.log(`File loaded. Length: ${content.length}`);

  const targets = ['1048576', '1600000', '1.6e6', '16e5', 'ceil', 'Anlas'];
  
  targets.forEach(t => {
    let idx = content.indexOf(t);
    // Print first 3 occurrences
    for(let i=0; i<3 && idx !== -1; i++) {
      console.log(`\nFound "${t}" at index ${idx}`);
      const start = Math.max(0, idx - 150);
      const end = Math.min(content.length, idx + 150);
      console.log('Context:', content.substring(start, end));
      idx = content.indexOf(t, idx + 1);
    }
    if (idx === -1 && content.indexOf(t) === -1) {
       console.log(`\n"${t}" not found.`);
    }
  });

} catch (e) {
  console.error('Error:', e.message);
}
