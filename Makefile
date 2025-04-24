run-coding:
	docker compose start process_supervisor
run-local-app:
	clear && HTTP_PORT=8888 cargo run
