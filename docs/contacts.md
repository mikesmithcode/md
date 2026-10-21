## Notes on Contact Management

- Calculate a Contact for i,j with all the mechanics.
- ContactManager stores this Contact in ContactManager's DashMap<ContactType, Contact>>. It is stored with i<j
- If j,i is needed ContactManager accesses a helper on Contact which inverts the signs appropriately but does not recalc, for use in force calcs. 
- When contact i,j no longer exists is_active is set to false. There will be one redundant calc if j,i is needed but this is not a big deal.
- Later during pruning anything with is_active false is removed from the DashMap. All remaining contacts have is_active set to false for next iteration of loop.
- When pruning and adding charge we add charge to both particles appropriately but we only need to look for i<j.
>

