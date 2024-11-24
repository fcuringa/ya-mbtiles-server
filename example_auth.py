def auth(path, **headers):
    """
    Authentication middleware.
    :param path: The request path
    :param headers: The request headers, only the ones requested in REQ_HEADERS parameter are included
    :return: True if the authentication is successful, False otherwise
    """
    print(path, headers)
    return True
