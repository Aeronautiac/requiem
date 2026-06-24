import type { ActionRequest, AppExecution } from "../bindings";

export interface Router {
    sendAction(action: ActionRequest): Promise<AppExecution>;
    quit(): void;
    // Add new commands here as they're added to the backend.
}

export const ROUTER_KEY = Symbol("router");
