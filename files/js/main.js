var tag_list = [];

var tag_input_name = "tag_input";


function setup()
{
    var tag_input_form = document.getElementById("tag_form");

    tag_input_form.addEventListener("submit", on_tag_submit, true);

    //Clear the tag input box
    var input_box = document.getElementById(tag_input_name);
    input_box.value = "";

    //Set up handlers for the buttons
    document.getElementById("button_next").addEventListener("click", request_next_img, false);
    document.getElementById("button_prev").addEventListener("click", request_prev_img, false);
    
    request_first_img();
}

function main()
{
    setup();
}


function add_tag(tag) 
{
    tag_list.push(tag);

    var tag_list_element = document.getElementById("tag_list");
    
    var new_element = document.createElement("li");
    new_element.innerHTML = tag;

    tag_list_element.insertBefore(new_element, tag_list_element.firstChild)
}

function on_tag_submit(e)
{
    //Don't submit the form which would reload the page
    e.preventDefault();

    //Find the text input box
    var input_box = document.getElementById(tag_input_name);

    add_tag(input_box.value);
    //reset the box to add more tags
    input_box.value = "";
}

ListRequestType = {
    CURRENT: "current",
    NEXT: "next",
    PREV: "prev",
}
function list_request(request_type)
{
    call_ajax("list?action=" + request_type, update_current_display);
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
