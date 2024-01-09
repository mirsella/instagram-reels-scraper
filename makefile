NAME=instagram-reels-scraper-emotion
all:
	docker rm $(NAME) || true
	docker build -t $(NAME) .
	docker run -d --name $(NAME) $(NAME)
