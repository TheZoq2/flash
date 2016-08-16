var tag_list = [];
var tag_input_id = "tag_input";
var tag_list_id = "tag_list"


function tag_manager(tag_list_element, tag_input_element)
{
	this.tag_list_element = tag_list_element;
	this.tag_input_element = tag_input_element;

	this.tag_list = [];
	
	//Updates the visual list of tags
	this.update_tag_dislpay = function()
	{
		//Clear the current contents of the list
		tag_list_element.innerHTML = "";

		//Add the current tags to the list
		for (var i = 0; i < this.tag_list.length; i++)
		{
			add_tag_to_display(tag_list[i]);
		}
	}

	//Adds a new tag to the list and updates the list view
	this.add_tag = function(tag)
	{
		var is_duplicate = false;
		
		for (var listed_tag in self.tag_list) {
			if(tag == listed_tag)
			{
				return;
			}
		}

		self.tag_list.push(tag);

		self.update_tag_dislpay();
	}

	this.add_tag_to_display = function(tag)
	{
		//Creating a new <li> element for the list
		var li_element = document.createElement("li");
		li_element.setAttribute("class", "tag_display");

		//Creating a new <div> to contain the contents of the <li>
		var new_div = document.createElement("div");

		//Insert the new div into the 
		li_element.insertBefore(new_div, li_element.firstChild);
		insert_element(new_div, li_element);

		//Add the remove button to the div
		insert_element(create_remove_button(function(){
			remove_tag(tag);
		}), new_div);

		var tag_text = document.createElement("p");
		tag_text.innerHTML = tag;
		insert_element(tag_text, new_div);

		//Add the new list element to the tag list
		insert_element(li_element, this.tag_list_element);
	}
}
//Clears the tag list display and adds all current tags
function update_tag_dislpay()
{
	var tag_list_element = document.getElementById(tag_list_id);
	tag_list_element.innerHTML = "";

	for (var i = 0; i < tag_list.length; i++)
	{
		add_tag_to_display(tag_list[i]);
	}
}
function add_tag(tag)
{
	//Ensure the tag is not in the list already
	var is_duplicate = false;
	for (var listed_tag in tag_list) {
		if(tag == listed_tag)
		{
			return;
		}
	}
	
	tag_list.push(tag);

	add_tag_to_display(tag);
}


function add_tag_to_display(tag)
{
	//Getting the list to add the tags to
	var tag_list_element = document.getElementById(tag_list_id);

	//Creating a new <li> element for the list
	var li_element = document.createElement("li");
	li_element.setAttribute("class", "tag_display");

	//Creating a new <div> to contain the contents of the <li>
	var new_div = document.createElement("div");

	//Insert the new div into the 
	li_element.insertBefore(new_div, li_element.firstChild);
	insert_element(new_div, li_element);

	//Add the remove button to the div
	insert_element(create_remove_button(function(){
		remove_tag(tag);
	}), new_div);

	var tag_text = document.createElement("p");
	tag_text.innerHTML = tag;
	insert_element(tag_text, new_div);

	//Add the new list element to the tag list
	insert_element(li_element, tag_list_element);
}

function on_tag_submit(e)
{
	 //Don't submit the form which would reload the page
	 e.preventDefault();

	 //Find the text input box
	 var input_box = document.getElementById(tag_input_id);

	 add_tag(input_box.value);
	 //reset the box to add more tags
	 input_box.value = "";
}


function create_remove_button(callback)
{
	var link = document.createElement("a");
	link.setAttribute("href", "#");
	link.setAttribute("class", "remove_button");
	link.addEventListener("click", callback, false);

	var img = document.createElement("img");
	img.setAttribute("src", "img/trashcan.svg");

	insert_element(img, link);
	return link;
}

//Inserts element into target
function insert_element(element, target)
{
	target.insertBefore(element, target.firstChild);
}

function remove_tag(tag)
{
	var index = tag_list.indexOf(tag);
	if(index != -1)
	{
		tag_list.splice(index, 1)
	}
	
	update_tag_dislpay();
}
