create-docker-secret:
	echo "Please enter your password: " && stty -echo && read DOCKER_PASSWORD && stty echo && echo $${DOCKER_PASSWORD}; \
	kubectl create secret docker-registry 7krasov-docker --docker-username=7krasov --docker-password=$${DOCKER_PASSWORD} --docker-email=7krasov@gmail.com
expose-deployment-port:
	kubectl expose deployment processes-supervisor-app-deployment --type=LoadBalancer --port=8080
#	kubectl expose deployment processes-supervisor-app-deployment --type=NodePort --port=8080
run-minikube-tunnel:
	minikube tunnel
