import os

output_file = "gpu_code.txt"
gpu_folder = "src/gpu"

with open(output_file, "w", encoding="utf-8") as out:
    for root, dirs, files in os.walk(gpu_folder):
        for filename in files:
            if filename.endswith((".rs", ".wgsl")):
                filepath = os.path.join(root, filename)
                rel_path = os.path.relpath(filepath, gpu_folder)
                out.write(f"{'='*60}\n")
                out.write(f"FILE: {rel_path}\n")
                out.write(f"{'='*60}\n\n")
                with open(filepath, "r", encoding="utf-8") as f:
                    out.write(f.read())
                out.write("\n\n")

print(f"Готово! Все коды собраны в {output_file}")
