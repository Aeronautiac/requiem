export class Flash {
    error: string | null = $state(null);
    success: string | null = $state(null);
    #error_timer: ReturnType<typeof setTimeout> | null = null;
    #success_timer: ReturnType<typeof setTimeout> | null = null;

    set_error(msg: string, duration = 3000) {
        if (this.#error_timer) clearTimeout(this.#error_timer);
        this.error = msg;
        this.#error_timer = setTimeout(() => { this.error = null; }, duration);
    }

    set_success(msg: string, duration = 3000) {
        if (this.#success_timer) clearTimeout(this.#success_timer);
        this.success = msg;
        this.#success_timer = setTimeout(() => { this.success = null; }, duration);
    }
}
