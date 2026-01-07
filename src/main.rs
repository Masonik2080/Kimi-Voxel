mod gpu;

fn main() {
    // вGPU версия - бесконечный terrain на шейдерах
    gpu::run();
    
    // CPU версия (закомментирована)
    // cpu::run();
}