import os

file_path = "/home/mur/workspace/novelAi/novelAI_offecial_folder/chunks/pages/_app-192d861dd8bcc7c3.js"
output_path = "formula_context.txt"

try:
    with open(output_path, "w", encoding="utf-8") as outfile:
        if not os.path.exists(file_path):
            outfile.write(f"File not found: {file_path}\n")
        else:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
                outfile.write(f"File loaded. Length: {len(content)}\n")

                found_something = False
                
                # Search for 16e5 (1,600,000)
                kw = "16e5"
                p1 = content.find(kw)
                if p1 != -1:
                    found_something = True
                    outfile.write(f"\n--- Found '{kw}' at {p1} ---\n")
                    start = max(0, p1 - 300)
                    end = min(len(content), p1 + 300)
                    snippet = content[start:end]
                    outfile.write(f"Context: {snippet}\n")
                else:
                    outfile.write(f"\n'{kw}' not found.\n")

                # Search for 39612 (The module ID)
                kw = "39612"
                p2 = content.find(kw)
                # We want the definition, so maybe look for "39612:" or "39612:("
                # But let's just find the first occurrence and iterate if needed
                start_p2 = 0
                count = 0
                while True:
                    p2 = content.find(kw, start_p2)
                    if p2 == -1 or count > 5:
                        break
                    
                    # Check if it looks like a key "39612:"
                    window = content[p2:p2+20]
                    outfile.write(f"\n--- Found '{kw}' match {count+1} at {p2}: {window} ---\n")
                    
                    if ":" in window:
                         # This looks promising
                         outfile.write("POTENTIAL DEFINITION:\n")
                         start = max(0, p2 - 50)
                         end = min(len(content), p2 + 500)
                         snippet = content[start:end]
                         outfile.write(f"Context: {snippet}\n")
                         found_something = True
                    
                    start_p2 = p2 + 1
                    count += 1

                if not found_something:
                    outfile.write("No relevant patterns found.\n")

except Exception as e:
    # If we can't write to file, well, we are stuck, but try to print to stdout as fallback (though it fails there too)
    print(f"Error: {e}")
