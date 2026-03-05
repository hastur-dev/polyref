// Version: 3.1.0

export declare function createApp(config: AppConfig): App;
export declare function version(): string;

export interface AppConfig {
    name: string;
    debug?: boolean;
}

export declare class App {
    name: string;
    start(port: number): void;
    stop(): void;
    use(middleware: Middleware): App;
}
