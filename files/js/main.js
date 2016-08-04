function setup()
{
	var tag_input_form = document.getElementById("tag_form");

	tag_input_form.addEventListener("submit", on_tag_submit, true);

	//Clear the tag input box
	var input_box = document.getElementById(tag_input_id);
	input_box.value = "";

	//Set up handlers for the buttons
	document.getElementById("button_next").addEventListener("click", request_next_img, false);
	document.getElementById("button_prev").addEventListener("click", request_prev_img, false);
	document.getElementById("button_save").addEventListener("click", send_save_request, false);

	request_first_img();
}

function main()
{
	setup();
}

ListRequestType = {
	CURRENT: "current",
	NEXT: "next",
	PREV: "prev",
	SAVE: "save",
}
function list_request(request_type, additional_variables = [])
{
	request_string = "list?action=" + request_type;
	for(var i = 0; i < additional_variables.length; i++)
	{
		request_string += "&" + additional_variables[i][0] + "=" + additional_variables[i][1];
	}
	call_ajax(request_string, update_current_display);

	console.log(request_string);
}
function request_first_img()
{
	//call_ajax("list?action=current", update_current_display);
	list_request(ListRequestType.CURRENT);
}
function request_next_img()
{
	list_request(ListRequestType.NEXT);
}
function request_prev_img()
{
	list_request(ListRequestType.PREV);
}

function send_save_request()
{
	//Generate json for the tag list
	var json_array = JSON.stringify(tag_list);

	var variables = [["tags", json_array]];
	
	//send the request off to the server
	list_request(ListRequestType.SAVE, variables);
}

function update_current_display(server_response)
{
	var content_div = document.getElementById("content");

	//clear the old stuff
	content_div.innerHTML = "";

	var json_response = JSON.parse(server_response);

	if(json_response.status == "ok")
	{
		//Create a new img element
		var img_element = document.createElement("img");

		img_element.setAttribute("src", json_response.file_path);
		img_element.setAttribute("id", "img_display")

		content_div.insertBefore(img_element, content_div.firstChild);

		//Update the cache thing
		if(json_response.next_file != "")
		{
			var cache_element = document.getElementById("img_cache");

			cache_element.setAttribute("src", json_response.next_file);
		}
	}
	else
	{
		content_div.inner_HTML = "No more images";
	}
}

function call_ajax(url, callback)
{
	var xmlhttp = new XMLHttpRequest();

	xmlhttp.onreadystatechange = function(){
		if(xmlhttp.readyState == XMLHttpRequest.DONE && xmlhttp.status == 200)
		{
			callback(xmlhttp.responseText);
		}
	}

	xmlhttp.open("GET", url, true);
	xmlhttp.send();
}
