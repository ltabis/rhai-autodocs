function openTab(evt, tabName) {
    const tabcontent = document.getElementsByClassName("tabcontent");

    for (let i = 0; i < tabcontent.length; i++) {
      if (tabcontent[i].id === tabName) {
        tabcontent[i].style.display = "none";
      }
    }

    const tablinks = document.getElementsByClassName("tablinks");

    for (let i = 0; i < tablinks.length; i++) {
      if (tablinks[i].id === "link-" + tabName) {
        tablinks[i].className = tablinks[i].className.replace(" active", "");
      }
    }
  
    document.getElementById(tabName).style.display = "block";
    evt.currentTarget.className += " active";
}